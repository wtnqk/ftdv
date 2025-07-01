use ratatui::style::Color;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

// Custom Color type that can be serialized/deserialized from config
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeColor(pub Color);

impl Serialize for ThemeColor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0 {
            Color::Reset => serializer.serialize_str("reset"),
            Color::Black => serializer.serialize_str("black"),
            Color::Red => serializer.serialize_str("red"),
            Color::Green => serializer.serialize_str("green"),
            Color::Yellow => serializer.serialize_str("yellow"),
            Color::Blue => serializer.serialize_str("blue"),
            Color::Magenta => serializer.serialize_str("magenta"),
            Color::Cyan => serializer.serialize_str("cyan"),
            Color::Gray => serializer.serialize_str("gray"),
            Color::DarkGray => serializer.serialize_str("dark_gray"),
            Color::LightRed => serializer.serialize_str("light_red"),
            Color::LightGreen => serializer.serialize_str("light_green"),
            Color::LightYellow => serializer.serialize_str("light_yellow"),
            Color::LightBlue => serializer.serialize_str("light_blue"),
            Color::LightMagenta => serializer.serialize_str("light_magenta"),
            Color::LightCyan => serializer.serialize_str("light_cyan"),
            Color::White => serializer.serialize_str("white"),
            Color::Indexed(n) => serializer.serialize_str(&format!("color{n}")),
            Color::Rgb(r, g, b) => serializer.serialize_str(&format!("#{r:02x}{g:02x}{b:02x}")),
        }
    }
}

struct ThemeColorVisitor;

impl Visitor<'_> for ThemeColorVisitor {
    type Value = ThemeColor;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a color name or hex code")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let color = match value.to_lowercase().as_str() {
            "reset" => Color::Reset,
            "black" => Color::Black,
            "red" => Color::Red,
            "green" => Color::Green,
            "yellow" => Color::Yellow,
            "blue" => Color::Blue,
            "magenta" => Color::Magenta,
            "cyan" => Color::Cyan,
            "gray" | "grey" => Color::Gray,
            "dark_gray" | "dark_grey" => Color::DarkGray,
            "light_red" => Color::LightRed,
            "light_green" => Color::LightGreen,
            "light_yellow" => Color::LightYellow,
            "light_blue" => Color::LightBlue,
            "light_magenta" => Color::LightMagenta,
            "light_cyan" => Color::LightCyan,
            "white" => Color::White,
            s if s.starts_with("color") => {
                let n = s[5..].parse::<u8>().map_err(de::Error::custom)?;
                Color::Indexed(n)
            }
            s if s.starts_with('#') && s.len() == 7 => {
                let r = u8::from_str_radix(&s[1..3], 16).map_err(de::Error::custom)?;
                let g = u8::from_str_radix(&s[3..5], 16).map_err(de::Error::custom)?;
                let b = u8::from_str_radix(&s[5..7], 16).map_err(de::Error::custom)?;
                Color::Rgb(r, g, b)
            }
            _ => return Err(de::Error::custom(format!("unknown color: {value}"))),
        };
        Ok(ThemeColor(color))
    }
}

impl<'de> Deserialize<'de> for ThemeColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(ThemeColorVisitor)
    }
}

impl From<ThemeColor> for Color {
    fn from(tc: ThemeColor) -> Self {
        tc.0
    }
}

impl Default for ThemeColor {
    fn default() -> Self {
        ThemeColor(Color::White)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorScheme {
    // File tree colors
    pub tree_line: ThemeColor,
    pub tree_selected_bg: ThemeColor,
    pub tree_selected_fg: ThemeColor,
    pub tree_directory: ThemeColor,
    pub tree_file: ThemeColor,

    // File status colors
    pub status_added: ThemeColor,
    pub status_removed: ThemeColor,
    pub status_modified: ThemeColor,

    // UI chrome colors
    pub border: ThemeColor,
    pub border_focused: ThemeColor,
    pub title: ThemeColor,
    pub status_bar_bg: ThemeColor,
    pub status_bar_fg: ThemeColor,

    // Text colors
    pub text_primary: ThemeColor,
    pub text_secondary: ThemeColor,
    pub text_dim: ThemeColor,

    // Background colors
    pub background: ThemeColor,
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self::dark_theme()
    }
}

impl ColorScheme {
    /// Default dark theme
    pub fn dark_theme() -> Self {
        Self {
            // File tree colors
            tree_line: ThemeColor(Color::DarkGray),
            tree_selected_bg: ThemeColor(Color::Rgb(50, 50, 70)),
            tree_selected_fg: ThemeColor(Color::Yellow),
            tree_directory: ThemeColor(Color::Blue),
            tree_file: ThemeColor(Color::White),

            // File status colors
            status_added: ThemeColor(Color::Green),
            status_removed: ThemeColor(Color::Red),
            status_modified: ThemeColor(Color::Yellow),

            // UI chrome colors
            border: ThemeColor(Color::DarkGray),
            border_focused: ThemeColor(Color::Cyan),
            title: ThemeColor(Color::Cyan),
            status_bar_bg: ThemeColor(Color::DarkGray),
            status_bar_fg: ThemeColor(Color::White),

            // Text colors
            text_primary: ThemeColor(Color::White),
            text_secondary: ThemeColor(Color::Gray),
            text_dim: ThemeColor(Color::DarkGray),

            // Background colors
            background: ThemeColor(Color::Black),
        }
    }
}

/// Theme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub colors: ColorScheme,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: "dark".to_string(),
            colors: ColorScheme::dark_theme(),
        }
    }
}
