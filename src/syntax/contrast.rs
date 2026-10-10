//! Explicit foreground adaptation for known RGB diff surfaces, performed before parsing.

use ratatui_core::style::Color;
use syntect::highlighting::Color as SyntaxColor;

use crate::{DiffTheme, SyntaxError};

pub(super) struct Contrast {
    backgrounds: [f64; 5],
}

impl Contrast {
    pub(super) fn new(theme: DiffTheme) -> Result<Self, SyntaxError> {
        let insert = theme.context.patch(theme.insert);
        let delete = theme.context.patch(theme.delete);
        let surfaces = [
            ("context", theme.context),
            ("insertion", insert),
            ("deletion", delete),
            ("inserted word", insert.patch(theme.insert_word)),
            ("deleted word", delete.patch(theme.delete_word)),
        ];
        let mut backgrounds = [0.0; 5];
        for (index, (surface, style)) in surfaces.into_iter().enumerate() {
            let Some(Color::Rgb(r, g, b)) = style.bg else {
                return Err(SyntaxError::UnsupportedBackground { surface });
            };
            backgrounds[index] = luminance(r, g, b);
        }
        Ok(Self { backgrounds })
    }

    pub(super) fn adjust(&self, original: SyntaxColor) -> Result<SyntaxColor, SyntaxError> {
        // Terminal-oriented themes can encode palette indexes through alpha. The adapter rejects
        // these during preparation; do not convert them into apparently valid RGB here.
        if original.a != 255 || self.accepts(original) {
            return Ok(original);
        }
        // Inspect a finite 8-bit mix rather than assuming a monotonic contrast predicate. Mixed
        // light/dark surfaces can admit an interior color while neither endpoint is acceptable.
        // Equal fractions prefer white. Channel order is preserved; perceptual hue is not promised.
        for fraction in 1..=255 {
            for target in [255, 0] {
                let candidate = SyntaxColor {
                    r: mix(original.r, target, fraction),
                    g: mix(original.g, target, fraction),
                    b: mix(original.b, target, fraction),
                    a: 255,
                };
                if self.accepts(candidate) {
                    return Ok(candidate);
                }
            }
        }
        Err(SyntaxError::InsufficientContrast {
            foreground: Color::Rgb(original.r, original.g, original.b),
        })
    }

    fn accepts(&self, color: SyntaxColor) -> bool {
        let foreground = luminance(color.r, color.g, color.b);
        self.backgrounds.iter().all(|&background| {
            (foreground.max(background) + 0.05) / (foreground.min(background) + 0.05) >= 4.5
        })
    }
}

fn mix(channel: u8, target: u8, fraction: u16) -> u8 {
    ((u16::from(channel) * (255 - fraction) + u16::from(target) * fraction + 127) / 255) as u8
}

fn luminance(r: u8, g: u8, b: u8) -> f64 {
    fn linear(channel: u8) -> f64 {
        let encoded = f64::from(channel) / 255.0;
        if encoded <= 0.04045 {
            encoded / 12.92
        } else {
            ((encoded + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
}
