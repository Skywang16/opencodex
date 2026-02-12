/*!
 * Theme system type definitions
 *
 * Contains all data structures and type definitions related to themes.
 */

use serde::{Deserialize, Serialize};

/// Theme configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThemeConfig {
    /// Terminal theme name, referencing files in themes/ directory
    pub terminal_theme: String,

    /// Light theme
    pub light_theme: String,

    /// Dark theme
    pub dark_theme: String,

    /// Follow system theme
    pub follow_system: bool,
}

/// Theme type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ThemeType {
    Light,
    Dark,
    Auto,
}

impl std::fmt::Display for ThemeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThemeType::Light => write!(f, "light"),
            ThemeType::Dark => write!(f, "dark"),
            ThemeType::Auto => write!(f, "auto"),
        }
    }
}

/// Theme definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    /// Theme name
    pub name: String,

    /// Theme type
    pub theme_type: ThemeType,

    /// ANSI colors
    pub ansi: AnsiColors,

    /// Bright ANSI colors
    pub bright: AnsiColors,

    /// Syntax highlighting
    pub syntax: SyntaxHighlight,

    /// UI colors
    pub ui: UIColors,
}

/// ANSI colors
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct AnsiColors {
    pub black: String,
    pub red: String,
    pub green: String,
    pub yellow: String,
    pub blue: String,
    pub magenta: String,
    pub cyan: String,
    pub white: String,
}

/// Syntax highlighting
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct SyntaxHighlight {
    /// Keywords
    pub keyword: String,

    /// Strings
    pub string: String,

    /// Comments
    pub comment: String,

    /// Numbers
    pub number: String,

    /// Functions
    pub function: String,

    /// Variables
    pub variable: String,

    /// Types
    pub type_name: String,

    /// Operators
    pub operator: String,
}

/// UI colors - new numeric hierarchy system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct UIColors {
    // Background color levels
    pub bg_100: String,
    pub bg_200: String,
    pub bg_300: String,
    pub bg_400: String,
    pub bg_500: String,
    pub bg_600: String,
    pub bg_700: String,

    // Border levels
    pub border_200: String,
    pub border_300: String,
    pub border_400: String,

    // Text levels
    pub text_100: String,
    pub text_200: String,
    pub text_300: String,
    pub text_400: String,
    pub text_500: String,

    // Status colors
    pub primary: String,
    pub primary_hover: String,
    pub primary_alpha: String,
    pub success: String,
    pub warning: String,
    pub error: String,
    pub info: String,

    // Interactive states
    pub hover: String,
    pub active: String,
    pub focus: String,
    pub selection: String,
}
