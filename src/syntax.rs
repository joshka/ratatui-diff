//! Optional bundled syntax preparation, independent of widget geometry and host I/O.

use std::error::Error;
use std::fmt;
use std::ops::Range;
use std::sync::OnceLock;

use ratatui_core::style::{Color, Modifier, Style};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Theme};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use two_face::theme::LazyThemeSet;

use crate::{DiffDocument, DiffLine, DiffTheme, Side, SourcePosition};

mod contrast;
use contrast::Contrast;

/// Bundled language grammars and a selected syntax theme, available with feature `syntax`.
///
/// Prepare styles outside the draw loop. This adapter owns tokenization and byte mapping; callers
/// supply language choices and optionally complete source snapshots. Engine types remain private.
/// Preparation is synchronous and may scan both complete sources. It has size limits, not a time
/// limit. A host can move this highlighter into a worker and return its owned [`SyntaxStyles`].
#[derive(Debug)]
pub struct SyntaxHighlighter {
    syntaxes: &'static SyntaxSet,
    theme: Theme,
    limits: SyntaxLimits,
}

impl SyntaxHighlighter {
    /// Load bundled grammars and select an exact theme name, such as `Nord`.
    ///
    /// Assets are shared across instances. Unknown names return [`SyntaxError::UnknownTheme`].
    /// Syntax backgrounds and underline are discarded; diff and interaction styles own those roles.
    /// Choose token colors compatible with all of the selected diff theme's backgrounds.
    ///
    /// ```
    /// use ratatui_diff::{Diff, DiffDocument, FileSyntax, SyntaxHighlighter, SyntaxSource};
    ///
    /// let old = "fn retry() { let timeout = 500; }\n";
    /// let new = "fn retry() { let timeout = 1500; }\n";
    /// let document = DiffDocument::from_text(old, new);
    /// let highlighter = SyntaxHighlighter::bundled("Nord")?;
    /// let styles = highlighter.prepare(
    ///     &document,
    ///     &[FileSyntax {
    ///         file: 0,
    ///         language: "rs",
    ///         source: SyntaxSource::Full { old, new },
    ///     }],
    /// )?;
    /// let diff = Diff::new(&document).syntax_styles(&styles)?;
    /// # Ok::<(), ratatui_diff::SyntaxError>(())
    /// ```
    pub fn bundled(theme: &str) -> Result<Self, SyntaxError> {
        let (syntaxes, themes) = bundled();
        let theme = themes
            .get(theme)
            .ok_or_else(|| SyntaxError::UnknownTheme(theme.to_owned()))?
            .clone();
        Ok(Self {
            syntaxes,
            theme,
            limits: SyntaxLimits::default(),
        })
    }

    /// Adapt token foregrounds to at least 4.5:1 contrast on this diff theme's five RGB surfaces.
    ///
    /// Explicitly changes syntax RGB; bundled colors are preserved unless this method is called.
    /// Keeps passing colors and mixes others toward white or black using the smallest passing
    /// 8-bit fraction (white wins ties). Channel order is preserved, not exact perceptual hue.
    /// This runs once on the selected engine theme, never during painting.
    ///
    /// Requires explicit RGB backgrounds for context, additions/deletions, and their word styles.
    /// Terminal-owned/unknown backgrounds return [`SyntaxError::UnsupportedBackground`]; a color
    /// with no passing mixture returns [`SyntaxError::InsufficientContrast`]. The guarantee applies
    /// only while rendering with the same diff backgrounds, before whitespace dimming and later
    /// search/selection overlays. Those overlays can replace or reverse foreground/background.
    pub fn contrast_with(mut self, theme: DiffTheme) -> Result<Self, SyntaxError> {
        let contrast = Contrast::new(theme)?;
        let foreground = self
            .theme
            .settings
            .foreground
            .unwrap_or(syntect::highlighting::Color::BLACK);
        self.theme.settings.foreground = Some(contrast.adjust(foreground)?);
        for item in &mut self.theme.scopes {
            if let Some(foreground) = item.style.foreground {
                item.style.foreground = Some(contrast.adjust(foreground)?);
            }
        }
        Ok(self)
    }

    /// Embedded syntax/theme license notices for applications distributing the bundled assets.
    ///
    /// Include these notices with a distributed application. They are independent of this crate's
    /// and its dependencies' Rust source licenses. Calling this method lazily loads upstream
    /// notices.
    pub fn acknowledgements() -> String {
        let notices = two_face::acknowledgement::listing();
        let mut text = String::from("# Bundled syntax and theme acknowledgements\n\n");
        for license in notices.for_syntaxes().iter().chain(notices.for_themes()) {
            if license.needs_acknowledgement() {
                license.write_md(&mut text);
            }
        }
        text
    }

    /// Names accepted by [`Self::bundled`], in the bundled set's order.
    pub fn themes() -> impl Iterator<Item = &'static str> {
        bundled().1.theme_names()
    }

    /// Bundled language names; extensions such as `rs` are also accepted by preparation.
    pub fn languages(&self) -> impl Iterator<Item = &str> {
        self.syntaxes
            .syntaxes()
            .iter()
            .map(|syntax| syntax.name.as_str())
    }

    /// Override finite preparation size limits. Zero rejects any work in that category.
    pub fn limits(mut self, limits: SyntaxLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Highlight selected files into an owned result bound to this document.
    ///
    /// Each file index may appear once; omitted files receive no syntax. Languages are extensions
    /// or case-insensitive grammar names. Full source must match every retained line, including
    /// CR and final-newline state. Retained input is explicitly best-effort: parsers restart after
    /// unavailable gaps and cannot know lexical state opened in omitted source.
    ///
    /// Errors reject the complete result: invalid/duplicate files, unknown languages, mismatched
    /// source, work-limit exhaustion, non-opaque foregrounds, and engine failures. No source is
    /// read from disk. Input strings are borrowed only during this call. Binary/metadata-only
    /// files produce no styles. Clones of the target document may use the result.
    pub fn prepare(
        &self,
        document: &DiffDocument,
        files: &[FileSyntax<'_>],
    ) -> Result<SyntaxStyles, SyntaxError> {
        self.preflight(document, files)?;
        let mut output = PreparedStyles {
            lines: Vec::new(),
            spans: 0,
            limit: self.limits.spans,
        };
        for input in files {
            let syntax = self
                .syntaxes
                .find_syntax_by_token(input.language)
                .ok_or_else(|| SyntaxError::UnknownLanguage {
                    file: input.file,
                    language: input.language.to_owned(),
                })?;
            for side in [Side::Old, Side::New] {
                let retained = retained_lines(document, input.file, side);
                match input.source {
                    SyntaxSource::Full { old, new } => {
                        let source = if side == Side::Old { old } else { new };
                        self.prepare_full(
                            input.file,
                            side,
                            syntax,
                            source,
                            &retained,
                            &mut output,
                        )?;
                    }
                    SyntaxSource::Retained => {
                        self.prepare_retained(input.file, side, syntax, &retained, &mut output)?;
                    }
                }
            }
        }
        output
            .lines
            .sort_unstable_by_key(|line| source_key(line.position));
        Ok(SyntaxStyles {
            document: document.id,
            lines: output.lines,
        })
    }

    fn preflight(
        &self,
        document: &DiffDocument,
        files: &[FileSyntax<'_>],
    ) -> Result<(), SyntaxError> {
        let mut seen = vec![false; document.files().len()];
        let mut bytes = 0usize;
        for input in files {
            let Some(already_seen) = seen.get_mut(input.file) else {
                return Err(SyntaxError::MissingFile(input.file));
            };
            if *already_seen {
                return Err(SyntaxError::DuplicateFile(input.file));
            }
            *already_seen = true;
            for side in [Side::Old, Side::New] {
                match input.source {
                    SyntaxSource::Full { old, new } => {
                        let source = if side == Side::Old { old } else { new };
                        for (index, text) in source.split_inclusive('\n').enumerate() {
                            let position = SourcePosition {
                                file: input.file,
                                side,
                                line: index + 1,
                            };
                            self.check_input(text.len(), position, &mut bytes)?;
                        }
                    }
                    SyntaxSource::Retained => {
                        for (number, line) in retained_lines(document, input.file, side) {
                            let position = SourcePosition {
                                file: input.file,
                                side,
                                line: number,
                            };
                            self.check_input(
                                line.text.len().saturating_add(usize::from(line.terminated)),
                                position,
                                &mut bytes,
                            )?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn check_input(
        &self,
        length: usize,
        position: SourcePosition,
        bytes: &mut usize,
    ) -> Result<(), SyntaxError> {
        if length > self.limits.line_bytes {
            return Err(SyntaxError::LimitExceeded {
                limit: SyntaxLimit::LineBytes,
                position,
            });
        }
        *bytes = bytes.saturating_add(length);
        if *bytes > self.limits.source_bytes {
            return Err(SyntaxError::LimitExceeded {
                limit: SyntaxLimit::SourceBytes,
                position,
            });
        }
        Ok(())
    }

    fn prepare_full(
        &self,
        file: usize,
        side: Side,
        syntax: &SyntaxReference,
        source: &str,
        retained: &[(usize, &DiffLine)],
        output: &mut PreparedStyles,
    ) -> Result<(), SyntaxError> {
        if retained.is_empty() {
            return Ok(());
        }
        let mut parser = HighlightLines::new(syntax, &self.theme);
        let mut next = 0;
        for (index, text) in source.split_inclusive('\n').enumerate() {
            let position = SourcePosition {
                file,
                side,
                line: index + 1,
            };
            let target = retained
                .get(next)
                .filter(|&&(number, _)| number == position.line);
            if let Some(&(_, line)) = target {
                if text.strip_suffix('\n').unwrap_or(text) != line.text
                    || text.ends_with('\n') != line.terminated
                {
                    return Err(SyntaxError::SourceMismatch(position));
                }
                next += 1;
            }
            let highlighted = parser
                .highlight_line(text, self.syntaxes)
                .map_err(|source| SyntaxError::Engine {
                    position,
                    source: Box::new(source),
                })?;
            if let Some(&(_, line)) = target {
                output.push(position, line.text.len(), highlighted)?;
            }
            if next == retained.len() {
                break;
            }
        }
        if let Some(&(line, _)) = retained.get(next) {
            return Err(SyntaxError::SourceMismatch(SourcePosition {
                file,
                side,
                line,
            }));
        }
        Ok(())
    }

    fn prepare_retained(
        &self,
        file: usize,
        side: Side,
        syntax: &SyntaxReference,
        retained: &[(usize, &DiffLine)],
        output: &mut PreparedStyles,
    ) -> Result<(), SyntaxError> {
        let mut parser = HighlightLines::new(syntax, &self.theme);
        let mut previous = None;
        let mut text = String::new();
        for &(number, line) in retained {
            if previous.is_some_and(|n| n + 1 != number) {
                parser = HighlightLines::new(syntax, &self.theme);
            }
            previous = Some(number);
            text.clear();
            text.push_str(&line.text);
            if line.terminated {
                text.push('\n');
            }
            let position = SourcePosition {
                file,
                side,
                line: number,
            };
            let highlighted = parser
                .highlight_line(&text, self.syntaxes)
                .map_err(|source| SyntaxError::Engine {
                    position,
                    source: Box::new(source),
                })?;
            output.push(position, line.text.len(), highlighted)?;
        }
        Ok(())
    }
}

fn bundled() -> &'static (SyntaxSet, LazyThemeSet) {
    static ASSETS: OnceLock<(SyntaxSet, LazyThemeSet)> = OnceLock::new();
    ASSETS.get_or_init(|| {
        (
            two_face::syntax::extra_newlines(),
            LazyThemeSet::from(two_face::theme::extra()),
        )
    })
}

/// Source availability for one file's syntax preparation.
#[derive(Debug, Clone, Copy)]
pub enum SyntaxSource<'a> {
    /// Complete original and modified UTF-8 snapshots. Use an empty absent side for additions or
    /// deletions. Each side's retained lines must match exactly; omitted text establishes lexical
    /// state but cannot be verified against the document.
    Full {
        /// Complete original source.
        old: &'a str,
        /// Complete modified source.
        new: &'a str,
    },
    /// Highlight retained lines only, restarting after unavailable source gaps. This can color
    /// multiline constructs incorrectly when their opening text is omitted. Hidden retained lines
    /// still participate; missing text is never reconstructed.
    Retained,
}

/// Language and available source for a file selected by its document index.
#[derive(Debug, Clone, Copy)]
pub struct FileSyntax<'a> {
    /// Zero-based index in [`DiffDocument::files`].
    pub file: usize,
    /// Grammar extension or case-insensitive name, such as `rs` or `Rust`.
    pub language: &'a str,
    /// Complete source or explicitly best-effort retained input.
    pub source: SyntaxSource<'a>,
}

/// Preparation size policy; limits do not impose a regex deadline or bound peak engine memory.
///
/// Defaults admit 64 MiB of total input, 64 KiB per line (including LF), and 1,000,000 coalesced
/// retained spans. Shared context counts once per side. Full inputs are preflighted in their
/// entirety, although parsing stops after the last retained line. Limits are resource policy, not
/// latency guarantees; prepare expensive inputs on a host worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntaxLimits {
    /// Maximum input bytes summed across selected files and both sides.
    pub source_bytes: usize,
    /// Maximum input bytes for a single source line, including a supplied LF.
    pub line_bytes: usize,
    /// Maximum stored ranges after adjacent equal styles are coalesced.
    pub spans: usize,
}

impl Default for SyntaxLimits {
    fn default() -> Self {
        Self {
            source_bytes: 64 * 1024 * 1024,
            line_bytes: 64 * 1024,
            spans: 1_000_000,
        }
    }
}

/// Immutable owned styles bound to a prepared document, available with feature `syntax`.
///
/// Produced by [`SyntaxHighlighter::prepare`], then borrowed with [`crate::Diff::syntax_styles`].
/// Contains no source or engine references and may be shared by independently navigated widgets.
/// Replacing it preserves geometry, search, selection, folds, and viewport. A new equal-text
/// document still requires preparation; document clones accept the original result. Unified context
/// uses new-side colors without old-side fallback; split context paints each side independently.
#[derive(Debug, Clone)]
pub struct SyntaxStyles {
    document: u64,
    lines: Vec<StyledLine>,
}

impl SyntaxStyles {
    pub(crate) fn validate_document(&self, document: &DiffDocument) -> Result<(), SyntaxError> {
        if self.document != document.id {
            return Err(SyntaxError::DocumentMismatch);
        }
        Ok(())
    }

    pub(crate) fn line(&self, position: SourcePosition) -> &[StyledSpan] {
        self.lines
            .binary_search_by_key(&source_key(position), |line| source_key(line.position))
            .map_or(&[], |index| &self.lines[index].spans)
    }
}

#[derive(Debug, Clone)]
struct StyledLine {
    position: SourcePosition,
    spans: Vec<StyledSpan>,
}

#[derive(Debug, Clone)]
pub(crate) struct StyledSpan {
    pub(crate) bytes: Range<usize>,
    pub(crate) style: Style,
}

struct PreparedStyles {
    lines: Vec<StyledLine>,
    spans: usize,
    limit: usize,
}

impl PreparedStyles {
    fn push(
        &mut self,
        position: SourcePosition,
        length: usize,
        highlighted: Vec<(syntect::highlighting::Style, &str)>,
    ) -> Result<(), SyntaxError> {
        let mut spans: Vec<StyledSpan> = Vec::new();
        let mut offset = 0;
        for (token_style, text) in highlighted {
            let end = (offset + text.len()).min(length);
            if offset < end {
                let style = terminal_style(token_style, position)?;
                if let Some(last) = spans.last_mut().filter(|span| span.style == style) {
                    last.bytes.end = end;
                } else {
                    if self.spans == self.limit {
                        return Err(SyntaxError::LimitExceeded {
                            limit: SyntaxLimit::Spans,
                            position,
                        });
                    }
                    self.spans += 1;
                    spans.push(StyledSpan {
                        bytes: offset..end,
                        style,
                    });
                }
            }
            offset += text.len();
        }
        if !spans.is_empty() {
            self.lines.push(StyledLine { position, spans });
        }
        Ok(())
    }
}

fn terminal_style(
    style: syntect::highlighting::Style,
    position: SourcePosition,
) -> Result<Style, SyntaxError> {
    let color = style.foreground;
    if color.a != 255 {
        return Err(SyntaxError::UnsupportedForeground(position));
    }
    let mut modifiers = Modifier::empty();
    if style.font_style.contains(FontStyle::BOLD) {
        modifiers.insert(Modifier::BOLD);
    }
    if style.font_style.contains(FontStyle::ITALIC) {
        modifiers.insert(Modifier::ITALIC);
    }
    Ok(Style::default()
        .fg(Color::Rgb(color.r, color.g, color.b))
        .add_modifier(modifiers))
}

fn retained_lines(document: &DiffDocument, file: usize, side: Side) -> Vec<(usize, &DiffLine)> {
    document.files()[file]
        .hunks
        .iter()
        .flat_map(|hunk| &hunk.lines)
        .filter_map(|line| {
            let number = if side == Side::Old {
                line.old
            } else {
                line.new
            };
            number.map(|number| (number, line))
        })
        .collect()
}

fn source_key(position: SourcePosition) -> (usize, u8, usize) {
    (
        position.file,
        u8::from(position.side == Side::New),
        position.line,
    )
}

/// Category of a preparation limit exceeded by an input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxLimit {
    /// Total source bytes across selected files and sides.
    SourceBytes,
    /// Bytes in one source line, including LF.
    LineBytes,
    /// Coalesced retained style ranges.
    Spans,
}

/// Failed syntax preparation or attachment; errors never install partial results.
#[derive(Debug)]
#[non_exhaustive]
pub enum SyntaxError {
    /// A bundled theme name was not found.
    UnknownTheme(String),
    /// A selected file's grammar token was not found.
    UnknownLanguage {
        /// Zero-based document file index.
        file: usize,
        /// Submitted language token.
        language: String,
    },
    /// File index is outside the target document.
    MissingFile(usize),
    /// File index appears more than once in preparation input.
    DuplicateFile(usize),
    /// Complete source differs from a retained line or ends before it.
    SourceMismatch(SourcePosition),
    /// Preparation input/output exceeds a configured finite limit.
    LimitExceeded {
        /// Limit category.
        limit: SyntaxLimit,
        /// Source line at which the limit was detected.
        position: SourcePosition,
    },
    /// A resolved foreground is non-opaque and cannot be represented faithfully in terminal cells.
    UnsupportedForeground(SourcePosition),
    /// The engine failed while processing a source line.
    Engine {
        /// Source line being processed, including omitted prefix lines for full inputs.
        position: SourcePosition,
        /// Underlying error, exposed without public engine types.
        source: Box<dyn Error + Send + Sync>,
    },
    /// Contrast adaptation requires a terminal-owned, absent, or non-RGB background.
    UnsupportedBackground {
        /// One of context, insertion, deletion, inserted word, or deleted word.
        surface: &'static str,
    },
    /// No 8-bit mixture toward white or black reaches 4.5:1 on all selected backgrounds.
    InsufficientContrast {
        /// Original syntax foreground.
        foreground: Color,
    },
    /// Styles belong to a different immutable document.
    DocumentMismatch,
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTheme(name) => write!(f, "unknown syntax theme: {name}"),
            Self::UnknownLanguage { file, language } => {
                write!(f, "unknown language {language:?} for file {file}")
            }
            Self::MissingFile(file) => write!(f, "missing syntax file {file}"),
            Self::DuplicateFile(file) => write!(f, "duplicate syntax file {file}"),
            Self::SourceMismatch(position) => write!(f, "syntax source mismatch at {position:?}"),
            Self::LimitExceeded { limit, position } => {
                write!(f, "syntax {limit:?} limit exceeded at {position:?}")
            }
            Self::UnsupportedForeground(position) => {
                write!(f, "non-opaque syntax foreground at {position:?}")
            }
            Self::Engine { position, source } => {
                write!(f, "syntax engine failed at {position:?}: {source}")
            }
            Self::UnsupportedBackground { surface } => {
                write!(f, "syntax contrast needs an RGB {surface} background")
            }
            Self::InsufficientContrast { foreground } => {
                write!(f, "no syntax contrast mixture passes for {foreground:?}")
            }
            Self::DocumentMismatch => f.write_str("syntax styles belong to another document"),
        }
    }
}

impl Error for SyntaxError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Engine { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}
