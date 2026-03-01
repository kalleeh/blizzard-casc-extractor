#![allow(dead_code)]

use casc_extractor::validation::regression_suite::SpriteMetadata;

pub fn sprite_metadata_64() -> SpriteMetadata {
    SpriteMetadata {
        width: 64,
        height: 64,
        frame_count: 1,
        format: "PNG".to_string(),
    }
}

pub fn sprite_metadata_32() -> SpriteMetadata {
    SpriteMetadata {
        width: 32,
        height: 32,
        frame_count: 1,
        format: "PNG".to_string(),
    }
}
