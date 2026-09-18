use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub theme: String,
    pub window: WindowConfig,
    pub font: FontConfig,
}

#[derive(Debug, Deserialize)]
pub struct WindowConfig {
    pub padding: u32,
}

#[derive(Debug, Deserialize)]
pub struct FontConfig {
    pub size: f32,
    pub cell_width: u32,
    pub cell_height: u32,
}

impl Config {
    pub fn load(path: &str) -> Self {
        let contents = fs::read_to_string(path).expect("Failed to read config");

        toml::from_str(&contents).expect("Failed to parse config")
    }
}

pub fn parse_color(color: &str) -> u32 {
    let hex = color.trim_start_matches('#');
    u32::from_str_radix(hex, 16).expect("Invalid color in config")
}
