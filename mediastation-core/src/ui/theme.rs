//! Theme system for MediaStation TUI

use ratatui::style::Color;

/// Theme definition with color schemes
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub background: Color,
    pub foreground: Color,
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub surface: Color,
    pub border: Color,
}

impl Theme {
    /// Get a theme by name
    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "dracula" => Self::dracula(),
            "gruvbox" => Self::gruvbox(),
            "nord" => Self::nord(),
            "rosepine" => Self::rosepine(),
            _ => Self::catppuccin(),
        }
    }

    /// Catppuccin Mocha theme (default)
    pub fn catppuccin() -> Self {
        Self {
            name: "catppuccin".to_string(),
            background: Color::Rgb(30, 30, 46),
            foreground: Color::Rgb(205, 214, 244),
            primary: Color::Rgb(137, 180, 250),
            secondary: Color::Rgb(166, 173, 200),
            accent: Color::Rgb(203, 166, 247),
            success: Color::Rgb(166, 227, 161),
            warning: Color::Rgb(249, 226, 175),
            error: Color::Rgb(243, 139, 168),
            surface: Color::Rgb(49, 50, 68),
            border: Color::Rgb(108, 112, 134),
        }
    }

    /// Dracula theme
    pub fn dracula() -> Self {
        Self {
            name: "dracula".to_string(),
            background: Color::Rgb(40, 42, 54),
            foreground: Color::Rgb(248, 248, 242),
            primary: Color::Rgb(189, 147, 249),
            secondary: Color::Rgb(139, 233, 253),
            accent: Color::Rgb(255, 121, 198),
            success: Color::Rgb(80, 250, 123),
            warning: Color::Rgb(241, 250, 140),
            error: Color::Rgb(255, 85, 85),
            surface: Color::Rgb(68, 71, 90),
            border: Color::Rgb(98, 114, 164),
        }
    }

    /// Gruvbox theme
    pub fn gruvbox() -> Self {
        Self {
            name: "gruvbox".to_string(),
            background: Color::Rgb(40, 40, 40),
            foreground: Color::Rgb(235, 219, 178),
            primary: Color::Rgb(184, 187, 38),
            secondary: Color::Rgb(156, 136, 100),
            accent: Color::Rgb(250, 80, 50),
            success: Color::Rgb(121, 192, 55),
            warning: Color::Rgb(254, 200, 50),
            error: Color::Rgb(204, 57, 50),
            surface: Color::Rgb(60, 56, 54),
            border: Color::Rgb(100, 80, 55),
        }
    }

    /// Nord theme
    pub fn nord() -> Self {
        Self {
            name: "nord".to_string(),
            background: Color::Rgb(46, 52, 64),
            foreground: Color::Rgb(216, 222, 233),
            primary: Color::Rgb(136, 192, 208),
            secondary: Color::Rgb(163, 190, 140),
            accent: Color::Rgb(129, 161, 193),
            success: Color::Rgb(163, 190, 140),
            warning: Color::Rgb(235, 203, 139),
            error: Color::Rgb(191, 97, 106),
            surface: Color::Rgb(59, 66, 82),
            border: Color::Rgb(76, 86, 106),
        }
    }

    /// Rosé Pine theme
    pub fn rosepine() -> Self {
        Self {
            name: "rosepine".to_string(),
            background: Color::Rgb(36, 33, 51),
            foreground: Color::Rgb(225, 214, 204),
            primary: Color::Rgb(235, 188, 186),
            secondary: Color::Rgb(198, 173, 179),
            accent: Color::Rgb(209, 170, 121),
            success: Color::Rgb(166, 227, 161),
            warning: Color::Rgb(237, 234, 183),
            error: Color::Rgb(231, 130, 132),
            surface: Color::Rgb(54, 50, 66),
            border: Color::Rgb(110, 104, 125),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::catppuccin()
    }
}
