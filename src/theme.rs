use crate::config::parse_color;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Theme {
    pub colors: ThemeColors,
    pub palette: ThemePalette,
}

#[derive(Debug, Deserialize)]
pub struct ThemeColors {
    pub background: String,
    pub foreground: String,
    pub cursor: String,
}

#[derive(Debug, Deserialize)]
pub struct ThemePalette {
    pub black: String,
    pub red: String,
    pub green: String,
    pub yellow: String,
    pub blue: String,
    pub magenta: String,
    pub cyan: String,
    pub white: String,
    pub bright_black: String,
    pub bright_red: String,
    pub bright_green: String,
    pub bright_yellow: String,
    pub bright_blue: String,
    pub bright_magenta: String,
    pub bright_cyan: String,
    pub bright_white: String,
}

impl Theme {
    pub fn load(path: &str) -> Self {
        let contents = fs::read_to_string(path).expect("Failed to read themes");
        toml::from_str(&contents).expect("Failed to parse themes")
    }
}

impl ThemePalette {
    pub fn color(&self, index: u8) -> u32 {
        match index {
            0 => parse_color(&self.black),
            1 => parse_color(&self.red),
            2 => parse_color(&self.green),
            3 => parse_color(&self.yellow),
            4 => parse_color(&self.blue),
            5 => parse_color(&self.magenta),
            6 => parse_color(&self.cyan),
            7 => parse_color(&self.white),
            8 => parse_color(&self.bright_black),
            9 => parse_color(&self.bright_red),
            10 => parse_color(&self.bright_green),
            11 => parse_color(&self.bright_yellow),
            12 => parse_color(&self.bright_blue),
            13 => parse_color(&self.bright_magenta),
            14 => parse_color(&self.bright_cyan),
            15 => parse_color(&self.bright_white),
            _ => unreachable!("Theme palette index must be between 0 and 15"),
        }
    }
}