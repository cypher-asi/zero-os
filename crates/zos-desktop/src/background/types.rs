use serde::{Deserialize, Serialize};

use super::shaders::{
    SHADER_BINARY, SHADER_DOTS, SHADER_GRAIN, SHADER_MIST_SMOKE, SHADER_MOSAIC, SHADER_PIXEL,
};

/// Available background types
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundType {
    /// Subtle film grain on dark background
    #[default]
    Grain,
    /// Animated misty/smoky atmosphere with glass overlay
    Mist,
    /// Small white pixelated dots in a regular grid pattern
    Dots,
    /// Moving colored pixels in angled patterns
    Pixel,
    /// Colorful cubes assembling into a grid with retro digital vibes
    Mosaic,
    /// Grid of 0s and 1s with domino-style flipping patterns
    Binary,
}

impl BackgroundType {
    /// Get all available background types
    pub fn all() -> &'static [BackgroundType] {
        &[
            BackgroundType::Grain,
            BackgroundType::Mist,
            BackgroundType::Dots,
            BackgroundType::Pixel,
            BackgroundType::Mosaic,
            BackgroundType::Binary,
        ]
    }

    /// Get the display name for this background
    pub fn name(&self) -> &'static str {
        match self {
            BackgroundType::Grain => "Grain",
            BackgroundType::Mist => "Myst",
            BackgroundType::Dots => "Dot Grid",
            BackgroundType::Pixel => "Pixels",
            BackgroundType::Mosaic => "Mosaic",
            BackgroundType::Binary => "Binary",
        }
    }

    /// Get the shader source for this background
    pub(crate) fn shader_source(&self) -> &'static str {
        match self {
            BackgroundType::Grain => SHADER_GRAIN,
            BackgroundType::Mist => SHADER_MIST_SMOKE,
            BackgroundType::Dots => SHADER_DOTS,
            BackgroundType::Pixel => SHADER_PIXEL,
            BackgroundType::Mosaic => SHADER_MOSAIC,
            BackgroundType::Binary => SHADER_BINARY,
        }
    }

    /// Parse from string ID (e.g., "grain", "mist", "dots", "pixel", "mosaic", "binary")
    pub fn from_id(id: &str) -> Option<Self> {
        match id.to_lowercase().as_str() {
            "grain" => Some(BackgroundType::Grain),
            "mist" => Some(BackgroundType::Mist),
            "dots" => Some(BackgroundType::Dots),
            "pixel" => Some(BackgroundType::Pixel),
            "mosaic" => Some(BackgroundType::Mosaic),
            "binary" => Some(BackgroundType::Binary),
            _ => None,
        }
    }

    /// Get the string ID for this background
    pub fn id(&self) -> &'static str {
        match self {
            BackgroundType::Grain => "grain",
            BackgroundType::Mist => "mist",
            BackgroundType::Dots => "dots",
            BackgroundType::Pixel => "pixel",
            BackgroundType::Mosaic => "mosaic",
            BackgroundType::Binary => "binary",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_background_type_default() {
        let bg: BackgroundType = Default::default();
        assert_eq!(bg, BackgroundType::Grain);
    }

    #[test]
    fn test_background_type_all() {
        let all = BackgroundType::all();
        assert_eq!(all.len(), 6);
        assert!(all.contains(&BackgroundType::Grain));
        assert!(all.contains(&BackgroundType::Mist));
        assert!(all.contains(&BackgroundType::Dots));
        assert!(all.contains(&BackgroundType::Pixel));
        assert!(all.contains(&BackgroundType::Mosaic));
        assert!(all.contains(&BackgroundType::Binary));
    }

    #[test]
    fn test_background_type_name() {
        assert_eq!(BackgroundType::Grain.name(), "Grain");
        assert_eq!(BackgroundType::Mist.name(), "Myst");
        assert_eq!(BackgroundType::Dots.name(), "Dot Grid");
        assert_eq!(BackgroundType::Pixel.name(), "Pixels");
        assert_eq!(BackgroundType::Mosaic.name(), "Mosaic");
        assert_eq!(BackgroundType::Binary.name(), "Binary");
    }

    #[test]
    fn test_background_type_id() {
        assert_eq!(BackgroundType::Grain.id(), "grain");
        assert_eq!(BackgroundType::Mist.id(), "mist");
        assert_eq!(BackgroundType::Dots.id(), "dots");
        assert_eq!(BackgroundType::Pixel.id(), "pixel");
        assert_eq!(BackgroundType::Mosaic.id(), "mosaic");
        assert_eq!(BackgroundType::Binary.id(), "binary");
    }

    #[test]
    fn test_background_type_from_id() {
        assert_eq!(
            BackgroundType::from_id("grain"),
            Some(BackgroundType::Grain)
        );
        assert_eq!(BackgroundType::from_id("mist"), Some(BackgroundType::Mist));
        assert_eq!(BackgroundType::from_id("dots"), Some(BackgroundType::Dots));
        assert_eq!(BackgroundType::from_id("pixel"), Some(BackgroundType::Pixel));
        assert_eq!(
            BackgroundType::from_id("mosaic"),
            Some(BackgroundType::Mosaic)
        );
        assert_eq!(
            BackgroundType::from_id("binary"),
            Some(BackgroundType::Binary)
        );
        assert_eq!(BackgroundType::from_id("invalid"), None);
    }

    #[test]
    fn test_background_type_from_id_case_insensitive() {
        assert_eq!(
            BackgroundType::from_id("GRAIN"),
            Some(BackgroundType::Grain)
        );
        assert_eq!(
            BackgroundType::from_id("Grain"),
            Some(BackgroundType::Grain)
        );
        assert_eq!(BackgroundType::from_id("MIST"), Some(BackgroundType::Mist));
        assert_eq!(BackgroundType::from_id("Mist"), Some(BackgroundType::Mist));
        assert_eq!(BackgroundType::from_id("DOTS"), Some(BackgroundType::Dots));
        assert_eq!(BackgroundType::from_id("Dots"), Some(BackgroundType::Dots));
        assert_eq!(BackgroundType::from_id("PIXEL"), Some(BackgroundType::Pixel));
        assert_eq!(BackgroundType::from_id("Pixel"), Some(BackgroundType::Pixel));
        assert_eq!(
            BackgroundType::from_id("MOSAIC"),
            Some(BackgroundType::Mosaic)
        );
        assert_eq!(
            BackgroundType::from_id("Mosaic"),
            Some(BackgroundType::Mosaic)
        );
        assert_eq!(
            BackgroundType::from_id("BINARY"),
            Some(BackgroundType::Binary)
        );
        assert_eq!(
            BackgroundType::from_id("Binary"),
            Some(BackgroundType::Binary)
        );
    }

    #[test]
    fn test_background_type_roundtrip() {
        for bg in BackgroundType::all() {
            let id = bg.id();
            let parsed = BackgroundType::from_id(id);
            assert_eq!(parsed, Some(*bg));
        }
    }

    #[test]
    fn test_background_type_shader_source_not_empty() {
        for bg in BackgroundType::all() {
            let source = bg.shader_source();
            assert!(
                !source.is_empty(),
                "Shader source for {:?} should not be empty",
                bg
            );
        }
    }

    #[test]
    fn test_background_type_serialize_deserialize() {
        let grain = BackgroundType::Grain;
        let serialized = serde_json::to_string(&grain).unwrap();
        assert_eq!(serialized, "\"grain\"");

        let deserialized: BackgroundType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, grain);

        let mist = BackgroundType::Mist;
        let serialized = serde_json::to_string(&mist).unwrap();
        assert_eq!(serialized, "\"mist\"");

        let deserialized: BackgroundType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, mist);
    }

    #[test]
    fn test_background_type_equality() {
        assert_eq!(BackgroundType::Grain, BackgroundType::Grain);
        assert_eq!(BackgroundType::Mist, BackgroundType::Mist);
        assert_eq!(BackgroundType::Dots, BackgroundType::Dots);
        assert_eq!(BackgroundType::Pixel, BackgroundType::Pixel);
        assert_eq!(BackgroundType::Mosaic, BackgroundType::Mosaic);
        assert_eq!(BackgroundType::Binary, BackgroundType::Binary);
        assert_ne!(BackgroundType::Grain, BackgroundType::Mist);
        assert_ne!(BackgroundType::Grain, BackgroundType::Dots);
        assert_ne!(BackgroundType::Mist, BackgroundType::Dots);
        assert_ne!(BackgroundType::Pixel, BackgroundType::Dots);
        assert_ne!(BackgroundType::Mosaic, BackgroundType::Dots);
        assert_ne!(BackgroundType::Binary, BackgroundType::Dots);
    }

    #[test]
    fn test_background_type_clone() {
        let bg = BackgroundType::Grain;
        let cloned = bg;
        assert_eq!(bg, cloned);
    }

    #[test]
    fn test_background_type_copy() {
        let bg = BackgroundType::Mist;
        let copied: BackgroundType = bg; // Copy trait
        assert_eq!(bg, copied);
    }

    #[test]
    fn test_background_type_debug() {
        let debug_grain = format!("{:?}", BackgroundType::Grain);
        assert!(debug_grain.contains("Grain"));

        let debug_mist = format!("{:?}", BackgroundType::Mist);
        assert!(debug_mist.contains("Mist"));
    }

    #[test]
    fn test_background_type_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(BackgroundType::Grain);
        set.insert(BackgroundType::Mist);
        set.insert(BackgroundType::Dots);
        set.insert(BackgroundType::Pixel);
        set.insert(BackgroundType::Mosaic);
        set.insert(BackgroundType::Binary);
        set.insert(BackgroundType::Grain); // Duplicate

        assert_eq!(set.len(), 6);
    }
}
