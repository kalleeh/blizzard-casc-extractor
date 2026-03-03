use crate::casc::FileEntry;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionTier {
    HD,
    HD2,
    SD,
    All,
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
use std::path::{Path, PathBuf};

/// Detect resolution tier from a file path string.
///
/// - HD2/anim/ -> HD2
/// - anim/ -> HD
/// - SD/ -> SD
/// - Other paths -> None
pub fn detect_resolution_tier(path: &str) -> Option<ResolutionTier> {
    let path_lower = path.to_lowercase();

    if path_lower.contains("hd2/anim/") || path_lower.contains("hd2\\anim\\") {
        return Some(ResolutionTier::HD2);
    }

    if path_lower.contains("anim/") || path_lower.contains("anim\\") {
        return Some(ResolutionTier::HD);
    }

    if path_lower.contains("sd/") || path_lower.contains("sd\\") {
        return Some(ResolutionTier::SD);
    }

    None
}

/// Filter a slice of `FileEntry` values by resolution tier.
pub fn filter_by_resolution<'a>(files: &'a [FileEntry], tier: ResolutionTier) -> Vec<&'a FileEntry> {
    let tier_name = format!("{}", tier);
    match tier {
        ResolutionTier::All => files.iter().collect(),
        _ => files
            .iter()
            .filter(|file| file.resolution_tier.as_deref() == Some(tier_name.as_str()))
            .collect(),
    }
}

/// Return the output directory for a given resolution tier.
pub fn get_output_path_for_tier(base_output_dir: &Path, tier: Option<ResolutionTier>) -> PathBuf {
    match tier {
        Some(ResolutionTier::HD) => base_output_dir.join("HD"),
        Some(ResolutionTier::HD2) => base_output_dir.join("HD2"),
        Some(ResolutionTier::SD) => base_output_dir.join("SD"),
        Some(ResolutionTier::All) | None => base_output_dir.to_path_buf(),
    }
}

/// Resolution tier handler for organizing sprite extraction by resolution
pub struct ResolutionHandler {
    #[allow(dead_code)]
    tier: ResolutionTier,
    #[allow(dead_code)]
    output_base: PathBuf,
}

impl ResolutionHandler {
    /// Create a new resolution handler
    pub fn new(tier: ResolutionTier, output_base: PathBuf) -> Self {
        Self {
            tier,
            output_base,
        }
    }

    /// Detect resolution tier from file path
    ///
    /// Analyzes the file path to determine which resolution tier it belongs to:
    /// - HD2/anim/ -> HD2
    /// - anim/ -> HD
    /// - SD/ -> SD
    /// - Other paths -> None
    pub fn detect_tier_from_path<P: AsRef<Path>>(path: P) -> Option<ResolutionTier> {
        let path_str = path.as_ref().to_string_lossy();
        detect_resolution_tier(&path_str)
    }

    #[cfg(test)]
    /// Check if a file should be processed based on resolution filter (test-only method)
    pub fn should_process_file<P: AsRef<Path>>(&self, file_path: P) -> bool {
        match self.tier {
            ResolutionTier::All => true,
            specific_tier => {
                match Self::detect_tier_from_path(&file_path) {
                    Some(detected_tier) => detected_tier == specific_tier,
                    None => false, // Unknown tier files are skipped when filtering
                }
            }
        }
    }

    #[cfg(test)]
    /// Get output directory for a specific resolution tier (test-only method)
    /// 
    /// Organizes output into subdirectories:
    /// - output/HD/ for HD sprites
    /// - output/HD2/ for HD2 sprites  
    /// - output/SD/ for SD sprites
    /// - output/ for All (no subdirectory)
    pub fn get_output_directory_for_tier(&self, detected_tier: ResolutionTier) -> PathBuf {
        match self.tier {
            ResolutionTier::All => {
                // When extracting all tiers, organize by detected tier
                match detected_tier {
                    ResolutionTier::HD => self.output_base.join("HD"),
                    ResolutionTier::HD2 => self.output_base.join("HD2"),
                    ResolutionTier::SD => self.output_base.join("SD"),
                    ResolutionTier::All => self.output_base.clone(), // Fallback
                }
            }
            _ => {
                // When extracting specific tier, use base output directory
                self.output_base.clone()
            }
        }
    }
}

/// Statistics for resolution tier filtering
#[derive(Debug, Clone, PartialEq)]
pub struct ResolutionFilterStats {
    pub target_tier: ResolutionTier,
    pub hd_count: u32,
    pub hd2_count: u32,
    pub sd_count: u32,
    pub unknown_count: u32,
    pub processed_count: u32,
    pub skipped_count: u32,
}

impl ResolutionFilterStats {
    #[cfg(test)]
    /// Update statistics with a processed file (test-only method)
    pub fn record_file<P: AsRef<Path>>(&mut self, file_path: P, processed: bool) {
        let detected_tier = ResolutionHandler::detect_tier_from_path(&file_path);
        
        match detected_tier {
            Some(ResolutionTier::HD) => self.hd_count += 1,
            Some(ResolutionTier::HD2) => self.hd2_count += 1,
            Some(ResolutionTier::SD) => self.sd_count += 1,
            Some(ResolutionTier::All) => {}, // Should not happen
            None => self.unknown_count += 1,
        }
        
        if processed {
            self.processed_count += 1;
        } else {
            self.skipped_count += 1;
        }
    }

    #[cfg(test)]
    /// Get total file count (test-only method)
    pub fn total_count(&self) -> u32 {
        self.hd_count + self.hd2_count + self.sd_count + self.unknown_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::path::PathBuf;

    // Property test generators
    fn resolution_tier_strategy() -> impl Strategy<Value = ResolutionTier> {
        prop_oneof![
            Just(ResolutionTier::HD),
            Just(ResolutionTier::HD2),
            Just(ResolutionTier::SD),
            Just(ResolutionTier::All),
        ]
    }

    fn hd_path_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("data/anim/terran/unit.anim".to_string()),
            Just("Data\\anim\\protoss\\building.anim".to_string()),
            Just("/path/to/anim/zerg/sprite.anim".to_string()),
            Just("C:\\Games\\StarCraft\\Data\\anim\\ui\\button.anim".to_string()),
        ]
    }

    fn hd2_path_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("data/HD2/anim/terran/unit.anim".to_string()),
            Just("Data\\HD2\\anim\\protoss\\building.anim".to_string()),
            Just("/path/to/HD2/anim/zerg/sprite.anim".to_string()),
            Just("C:\\Games\\StarCraft\\Data\\HD2\\anim\\ui\\button.anim".to_string()),
        ]
    }

    fn sd_path_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("data/SD/terran/unit.anim".to_string()),
            Just("Data\\SD\\protoss\\building.anim".to_string()),
            Just("/path/to/SD/zerg/sprite.anim".to_string()),
            Just("C:\\Games\\StarCraft\\Data\\SD\\ui\\button.anim".to_string()),
        ]
    }

    fn unknown_path_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("data/other/file.anim".to_string()),
            Just("Data\\misc\\sprite.anim".to_string()),
            Just("/path/to/unknown/file.anim".to_string()),
            Just("C:\\Games\\StarCraft\\Data\\temp\\file.anim".to_string()),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        #[test]
        // **Feature: casc-sprite-extractor, Property 16: Resolution Tier Categorization**
        // **Validates: Requirements 4.1**
        fn property_16_resolution_tier_categorization(
            hd_path in hd_path_strategy(),
            hd2_path in hd2_path_strategy(),
            sd_path in sd_path_strategy(),
            unknown_path in unknown_path_strategy()
        ) {
            // HD paths should be detected as HD
            prop_assert_eq!(
                ResolutionHandler::detect_tier_from_path(&hd_path),
                Some(ResolutionTier::HD),
                "HD path '{}' should be detected as HD tier", hd_path
            );

            // HD2 paths should be detected as HD2
            prop_assert_eq!(
                ResolutionHandler::detect_tier_from_path(&hd2_path),
                Some(ResolutionTier::HD2),
                "HD2 path '{}' should be detected as HD2 tier", hd2_path
            );

            // SD paths should be detected as SD
            prop_assert_eq!(
                ResolutionHandler::detect_tier_from_path(&sd_path),
                Some(ResolutionTier::SD),
                "SD path '{}' should be detected as SD tier", sd_path
            );

            // Unknown paths should return None
            prop_assert_eq!(
                ResolutionHandler::detect_tier_from_path(&unknown_path),
                None,
                "Unknown path '{}' should not be detected as any tier", unknown_path
            );
        }

        #[test]
        // **Feature: casc-sprite-extractor, Property 17: Resolution Directory Organization**
        // **Validates: Requirements 4.2**
        fn property_17_resolution_directory_organization(
            tier in resolution_tier_strategy(),
            detected_tier in resolution_tier_strategy().prop_filter("Not All", |t| *t != ResolutionTier::All)
        ) {
            let output_base = PathBuf::from("test_output");
            let handler = ResolutionHandler::new(tier, output_base.clone());
            
            let output_dir = handler.get_output_directory_for_tier(detected_tier);
            
            match tier {
                ResolutionTier::All => {
                    // When extracting all tiers, should organize by detected tier
                    let expected = match detected_tier {
                        ResolutionTier::HD => output_base.join("HD"),
                        ResolutionTier::HD2 => output_base.join("HD2"),
                        ResolutionTier::SD => output_base.join("SD"),
                        ResolutionTier::All => output_base.clone(),
                    };
                    prop_assert_eq!(output_dir, expected);
                }
                _ => {
                    // When extracting specific tier, should use base directory
                    prop_assert_eq!(output_dir, output_base);
                }
            }
        }

        #[test]
        // **Feature: casc-sprite-extractor, Property 18: Resolution Filter Application**
        // **Validates: Requirements 4.5**
        fn property_18_resolution_filter_application(
            filter_tier in resolution_tier_strategy(),
            hd_path in hd_path_strategy(),
            hd2_path in hd2_path_strategy(),
            sd_path in sd_path_strategy(),
            unknown_path in unknown_path_strategy()
        ) {
            let output_base = PathBuf::from("test_output");
            let handler = ResolutionHandler::new(filter_tier, output_base);
            
            // Test HD path filtering
            let should_process_hd = handler.should_process_file(&hd_path);
            let expected_hd = match filter_tier {
                ResolutionTier::All => true,
                ResolutionTier::HD => true,
                _ => false,
            };
            prop_assert_eq!(should_process_hd, expected_hd, 
                "HD path '{}' processing with filter {:?} should be {}", hd_path, filter_tier, expected_hd);

            // Test HD2 path filtering
            let should_process_hd2 = handler.should_process_file(&hd2_path);
            let expected_hd2 = match filter_tier {
                ResolutionTier::All => true,
                ResolutionTier::HD2 => true,
                _ => false,
            };
            prop_assert_eq!(should_process_hd2, expected_hd2,
                "HD2 path '{}' processing with filter {:?} should be {}", hd2_path, filter_tier, expected_hd2);

            // Test SD path filtering
            let should_process_sd = handler.should_process_file(&sd_path);
            let expected_sd = match filter_tier {
                ResolutionTier::All => true,
                ResolutionTier::SD => true,
                _ => false,
            };
            prop_assert_eq!(should_process_sd, expected_sd,
                "SD path '{}' processing with filter {:?} should be {}", sd_path, filter_tier, expected_sd);

            // Test unknown path filtering
            let should_process_unknown = handler.should_process_file(&unknown_path);
            let expected_unknown = match filter_tier {
                ResolutionTier::All => true,
                _ => false, // Unknown paths are skipped when filtering
            };
            prop_assert_eq!(should_process_unknown, expected_unknown,
                "Unknown path '{}' processing with filter {:?} should be {}", unknown_path, filter_tier, expected_unknown);
        }
    }

    #[test]
    fn test_resolution_filter_stats() {
        let mut stats = ResolutionFilterStats {
            target_tier: ResolutionTier::All,
            hd_count: 0,
            hd2_count: 0,
            sd_count: 0,
            unknown_count: 0,
            processed_count: 0,
            skipped_count: 0,
        };

        // Record some files
        stats.record_file("data/anim/terran/unit.anim", true);
        stats.record_file("data/HD2/anim/protoss/building.anim", true);
        stats.record_file("data/SD/zerg/sprite.anim", false);
        stats.record_file("data/other/file.anim", false);

        assert_eq!(stats.hd_count, 1);
        assert_eq!(stats.hd2_count, 1);
        assert_eq!(stats.sd_count, 1);
        assert_eq!(stats.unknown_count, 1);
        assert_eq!(stats.processed_count, 2);
        assert_eq!(stats.skipped_count, 2);
        assert_eq!(stats.total_count(), 4);
    }

    #[test]
    fn test_path_detection_case_insensitive() {
        // Test case insensitive detection
        assert_eq!(
            ResolutionHandler::detect_tier_from_path("DATA/ANIM/TERRAN/UNIT.ANIM"),
            Some(ResolutionTier::HD)
        );
        assert_eq!(
            ResolutionHandler::detect_tier_from_path("data/hd2/anim/protoss/building.anim"),
            Some(ResolutionTier::HD2)
        );
        assert_eq!(
            ResolutionHandler::detect_tier_from_path("Data\\SD\\Zerg\\Sprite.anim"),
            Some(ResolutionTier::SD)
        );
    }

    #[test]
    fn test_cross_platform_paths() {
        // Test both Unix and Windows path separators
        assert_eq!(
            ResolutionHandler::detect_tier_from_path("data/anim/file.anim"),
            Some(ResolutionTier::HD)
        );
        assert_eq!(
            ResolutionHandler::detect_tier_from_path("data\\anim\\file.anim"),
            Some(ResolutionTier::HD)
        );
        assert_eq!(
            ResolutionHandler::detect_tier_from_path("data/HD2/anim/file.anim"),
            Some(ResolutionTier::HD2)
        );
        assert_eq!(
            ResolutionHandler::detect_tier_from_path("data\\HD2\\anim\\file.anim"),
            Some(ResolutionTier::HD2)
        );
    }
}