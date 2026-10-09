//! Styles are independent of comparison and layout.

use ratatui_core::style::{Color, Modifier, Style};

/// Styles for each diff role. Fields can be overridden independently.
///
/// [`Default`] selects [`dark`](Self::dark). Presets leave the terminal background unspecified;
/// their appearance depends on the terminal palette. Word styles are patched over line styles,
/// while gutters have a separate style. Changing a widget's theme preserves its layout cache.
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

    /// Inserted content and blank cells in its pane, excluding the gutter.
    pub insert: Style,

    /// Deleted content and blank cells in its pane, excluding the gutter.
    pub delete: Style,

    /// Inline insertion emphasis, patched over the line style.
    pub insert_word: Style,

    /// Inline deletion emphasis, patched over the line style.
    pub delete_word: Style,

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
            header: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            gutter: Style::default().fg(Color::DarkGray),
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
            header: Style::default().add_modifier(Modifier::BOLD),
            gutter: Style::default(),
        }
    }
}
