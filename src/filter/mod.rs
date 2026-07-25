//! Filter option types used by the CLI and the JSON config file.
//!
//! Path/pattern filtering itself lives in `main.rs::passes_filter`. The
//! `FileFilter` engine that used to live here was never called by the binary —
//! only by its own tests — and has been removed.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormatFilterOption {
    Png,
    Jpeg,
    Images,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnityFilterMode {
    Point,
    Bilinear,
    Trilinear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnityWrapMode {
    Clamp,
    Repeat,
    Mirror,
}

impl std::str::FromStr for FormatFilterOption {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "png" => Ok(FormatFilterOption::Png),
            "jpeg" | "jpg" => Ok(FormatFilterOption::Jpeg),
            "images" => Ok(FormatFilterOption::Images),
            "all" => Ok(FormatFilterOption::All),
            _ => Err(format!("Invalid format filter: {}", s)),
        }
    }
}

impl std::fmt::Display for FormatFilterOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatFilterOption::Png => write!(f, "PNG"),
            FormatFilterOption::Jpeg => write!(f, "JPEG"),
            FormatFilterOption::Images => write!(f, "Images"),
            FormatFilterOption::All => write!(f, "All"),
        }
    }
}

impl std::str::FromStr for UnityFilterMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "point" => Ok(UnityFilterMode::Point),
            "bilinear" => Ok(UnityFilterMode::Bilinear),
            "trilinear" => Ok(UnityFilterMode::Trilinear),
            _ => Err(format!("Invalid Unity filter mode: {}", s)),
        }
    }
}

impl std::fmt::Display for UnityFilterMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnityFilterMode::Point => write!(f, "Point"),
            UnityFilterMode::Bilinear => write!(f, "Bilinear"),
            UnityFilterMode::Trilinear => write!(f, "Trilinear"),
        }
    }
}

impl std::str::FromStr for UnityWrapMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "clamp" => Ok(UnityWrapMode::Clamp),
            "repeat" => Ok(UnityWrapMode::Repeat),
            "mirror" => Ok(UnityWrapMode::Mirror),
            _ => Err(format!("Invalid Unity wrap mode: {}", s)),
        }
    }
}

impl std::fmt::Display for UnityWrapMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnityWrapMode::Clamp => write!(f, "Clamp"),
            UnityWrapMode::Repeat => write!(f, "Repeat"),
            UnityWrapMode::Mirror => write!(f, "Mirror"),
        }
    }
}
