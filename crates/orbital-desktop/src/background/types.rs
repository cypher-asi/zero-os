use serde::{Deserialize, Serialize};

use super::shaders::{SHADER_GRAIN, SHADER_MIST_SMOKE};

/// Available background types
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundType {
    /// Subtle film grain on dark background
    Grain,
    /// Animated misty/smoky atmosphere with glass overlay
    Mist,
}

impl BackgroundType {
    /// Get all available background types
    pub fn all() -> &'static [BackgroundType] {
        &[BackgroundType::Grain, BackgroundType::Mist]
    }

    /// Get the display name for this background
    pub fn name(&self) -> &'static str {
        match self {
            BackgroundType::Grain => "Film Grain",
            BackgroundType::Mist => "Misty Smoke",
        }
    }

    /// Get the shader source for this background
    pub(crate) fn shader_source(&self) -> &'static str {
        match self {
            BackgroundType::Grain => SHADER_GRAIN,
            BackgroundType::Mist => SHADER_MIST_SMOKE,
        }
    }

    /// Parse from string ID (e.g., "grain", "mist")
    pub fn from_id(id: &str) -> Option<Self> {
        match id.to_lowercase().as_str() {
            "grain" => Some(BackgroundType::Grain),
            "mist" => Some(BackgroundType::Mist),
            _ => None,
        }
    }

    /// Get the string ID for this background
    pub fn id(&self) -> &'static str {
        match self {
            BackgroundType::Grain => "grain",
            BackgroundType::Mist => "mist",
        }
    }
}

impl Default for BackgroundType {
    fn default() -> Self {
        BackgroundType::Grain
    }
}
