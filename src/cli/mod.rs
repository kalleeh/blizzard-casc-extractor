use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionTier {
    HD,
    HD2,
    SD,
    All,
}

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

impl std::str::FromStr for ResolutionTier {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "hd" => Ok(ResolutionTier::HD),
            "hd2" => Ok(ResolutionTier::HD2),
            "sd" => Ok(ResolutionTier::SD),
            "all" => Ok(ResolutionTier::All),
            _ => Err(format!("Invalid resolution tier: {}", s)),
        }
    }
}

impl std::fmt::Display for ResolutionTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolutionTier::HD => write!(f, "HD"),
            ResolutionTier::HD2 => write!(f, "HD2"),
            ResolutionTier::SD => write!(f, "SD"),
            ResolutionTier::All => write!(f, "All"),
        }
    }
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
