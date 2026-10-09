//! Styles are independent of comparison and layout.

use ratatui_core::style::{Color, Modifier, Style};

/// Styles for each diff role. Fields can be overridden independently.
///
/// [`Default`] selects [`dark`](Self::dark), which uses terminal palette colors.
/// [`aardvark_ink`](Self::aardvark_ink) sets explicit RGB foregrounds and backgrounds.
/// Word styles are patched over line styles, followed by search emphasis. Gutters have a separate
/// style. Changing a widget's theme preserves its layout cache.
///
/// # Example
///
/// ```
/// use ratatui_core::style::{Color, Style};
/// use ratatui_diff::DiffTheme;
///
/// let theme = DiffTheme {
///     header: Style::default().fg(Color::Yellow),
///     ..DiffTheme::monochrome()
/// };
/// assert_eq!(theme.header.fg, Some(Color::Yellow));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffTheme {
    /// Unchanged content and blank cells.
    pub context: Style,

    /// Inserted content, change markers, and blank cells, excluding the number gutter.
    pub insert: Style,

    /// Deleted content, change markers, and blank cells, excluding the number gutter.
    pub delete: Style,

    /// Inline insertion emphasis, patched over the line style.
    pub insert_word: Style,

    /// Inline deletion emphasis, patched over the line style.
    pub delete_word: Style,

    /// Literal search emphasis, patched after word styles and synthetic whitespace dimming.
    pub search_match: Style,

    /// Selected search occurrence, patched over line and word styles.
    pub search_active: Style,

    /// File and hunk headers.
    pub header: Style,

    /// Line numbers and separators.
    pub gutter: Style,
}

impl Default for DiffTheme {
    fn default() -> Self {
        Self::dark()
    }
}

impl DiffTheme {
    /// A dark-background preset using terminal palette colors.
    pub fn dark() -> Self {
        Self {
            context: Style::default(),
            insert: Style::default().fg(Color::Green),
            delete: Style::default().fg(Color::Red),
            insert_word: Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
            delete_word: Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
            search_match: Style::default().add_modifier(Modifier::UNDERLINED),
            search_active: Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD),
            header: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            gutter: Style::default().fg(Color::DarkGray),
        }
    }

    /// An RGB preset complementing the Aardvark Ink terminal palette.
    ///
    /// Uses green additions, red deletions, and stronger backgrounds for changed words.
    /// Sets the widget's foreground and background; the host configures the surrounding terminal.
    /// Requires truecolor support. Use [`Self::dark`] for terminal palette colors or
    /// [`Self::monochrome`] for markers and underlined word highlights without explicit colors.
    ///
    /// ```
    /// use ratatui_diff::{Diff, DiffDocument, DiffTheme};
    ///
    /// let document = DiffDocument::from_text("old\n", "new\n");
    /// let diff = Diff::new(&document).theme(DiffTheme::aardvark_ink());
    /// ```
    pub fn aardvark_ink() -> Self {
        // Aardvark Ink's bright green/red/cyan retain contrast on tinted ink backgrounds.
        Self {
            context: Style::default()
                .fg(Color::Rgb(0xb4, 0xbc, 0xca))
                .bg(Color::Rgb(0x0f, 0x14, 0x1f)),
            insert: Style::default()
                .fg(Color::Rgb(0x75, 0xcf, 0x84))
                .bg(Color::Rgb(0x16, 0x2b, 0x25)),
            delete: Style::default()
                .fg(Color::Rgb(0xe4, 0x83, 0x83))
                .bg(Color::Rgb(0x30, 0x20, 0x2a)),
            insert_word: Style::default()
                .bg(Color::Rgb(0x25, 0x48, 0x35))
                .add_modifier(Modifier::BOLD),
            delete_word: Style::default()
                .bg(Color::Rgb(0x4b, 0x25, 0x30))
                .add_modifier(Modifier::BOLD),
            search_match: Style::default().add_modifier(Modifier::UNDERLINED),
            search_active: Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD),
            header: Style::default()
                .fg(Color::Rgb(0x52, 0xc4, 0xc0))
                .bg(Color::Rgb(0x0f, 0x14, 0x1f))
                .add_modifier(Modifier::BOLD),
            // Quiet numbers inherit row backgrounds; +/- markers use their source row style.
            gutter: Style::default().fg(Color::Rgb(0x6f, 0x7a, 0x8f)),
        }
    }

    /// A light-background preset with RGB text colors; retains the dark preset's word backgrounds.
    pub fn light() -> Self {
        Self {
            insert: Style::default().fg(Color::Rgb(0, 100, 0)),
            delete: Style::default().fg(Color::Rgb(160, 0, 0)),
            gutter: Style::default().fg(Color::Rgb(100, 100, 100)),
            ..Self::dark()
        }
    }

    /// A color-free preset using bold headers and underlined word highlights. Markers remain
    /// visible.
    pub fn monochrome() -> Self {
        Self {
            context: Style::default(),
            insert: Style::default(),
            delete: Style::default(),
            insert_word: Style::default().add_modifier(Modifier::UNDERLINED),
            delete_word: Style::default().add_modifier(Modifier::UNDERLINED),
            search_match: Style::default().add_modifier(Modifier::UNDERLINED),
            search_active: Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD),
            header: Style::default().add_modifier(Modifier::BOLD),
            gutter: Style::default(),
        }
    }
}
