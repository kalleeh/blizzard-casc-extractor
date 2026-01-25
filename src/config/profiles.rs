//! Configuration Profiles for CASC Sprite Extractor
//! 
//! This module provides configuration profile management, allowing users to:
//! - Create reusable configuration profiles
//! - Save and load profiles from disk
//! - Apply predefined profiles for common use cases
//! - Manage profile collections and inheritance

use super::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};

/// Configuration profile manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationProfileManager {
    /// Available profiles
    pub profiles: HashMap<String, ConfigurationProfile>,
    
    /// Default profile name
    pub default_profile: Option<String>,
    
    /// Profile storage directory
    pub profile_directory: PathBuf,
}

/// Individual configuration profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationProfile {
    /// Profile name
    pub name: String,
    
    /// Profile description
    pub description: String,
    
    /// Profile version
    pub version: String,
    
    /// Profile author
    pub author: Option<String>,
    
    /// Profile tags for categorization
    pub tags: Vec<String>,
    
    /// Base profile to inherit from
    pub inherits_from: Option<String>,
    
    /// Configuration settings
    pub config: ExtractionConfig,
    
    /// Profile metadata
    pub metadata: ProfileMetadata,
}

/// Profile metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileMetadata {
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    
    /// Last modified timestamp
    pub modified_at: chrono::DateTime<chrono::Utc>,
    
    /// Usage count
    pub usage_count: u64,
    
    /// Last used timestamp
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
    
    /// Profile rating (1-5 stars)
    pub rating: Option<u8>,
    
    /// User notes
    pub notes: Option<String>,
}

impl Default for ConfigurationProfileManager {
    fn default() -> Self {
        let profile_directory = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("casc-extractor")
            .join("profiles");
        
        Self {
            profiles: HashMap::new(),
            default_profile: None,
            profile_directory,
        }
    }
}

impl ConfigurationProfileManager {
    /// Create a new profile manager
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create a profile manager with custom profile directory
    pub fn with_profile_directory<P: AsRef<Path>>(directory: P) -> Self {
        Self {
            profiles: HashMap::new(),
            default_profile: None,
            profile_directory: directory.as_ref().to_path_buf(),
        }
    }
    
    /// Load profiles from the profile directory
    pub fn load_profiles(&mut self) -> Result<()> {
        if !self.profile_directory.exists() {
            std::fs::create_dir_all(&self.profile_directory)
                .with_context(|| format!("Failed to create profile directory: {:?}", self.profile_directory))?;
            
            // Create default profiles
            self.create_default_profiles()?;
            return Ok(());
        }
        
        let entries = std::fs::read_dir(&self.profile_directory)
            .with_context(|| format!("Failed to read profile directory: {:?}", self.profile_directory))?;
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match self.load_profile_from_file(&path) {
                    Ok(profile) => {
                        self.profiles.insert(profile.name.clone(), profile);
                    }
                    Err(e) => {
                        log::warn!("Failed to load profile from {:?}: {}", path, e);
                    }
                }
            }
        }
        
        // If no profiles were loaded, create defaults
        if self.profiles.is_empty() {
            self.create_default_profiles()?;
        }
        
        Ok(())
    }
    
    /// Save all profiles to disk
    pub fn save_profiles(&self) -> Result<()> {
        if !self.profile_directory.exists() {
            std::fs::create_dir_all(&self.profile_directory)
                .with_context(|| format!("Failed to create profile directory: {:?}", self.profile_directory))?;
        }
        
        for profile in self.profiles.values() {
            self.save_profile_to_file(profile)?;
        }
        
        Ok(())
    }
    
    /// Load a single profile from file
    fn load_profile_from_file<P: AsRef<Path>>(&self, path: P) -> Result<ConfigurationProfile> {
        let content = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read profile file: {:?}", path.as_ref()))?;
        
        let profile: ConfigurationProfile = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse profile file: {:?}", path.as_ref()))?;
        
        Ok(profile)
    }
    
    /// Save a single profile to file
    fn save_profile_to_file(&self, profile: &ConfigurationProfile) -> Result<()> {
        let filename = format!("{}.json", profile.name.replace(' ', "_").to_lowercase());
        let path = self.profile_directory.join(filename);
        
        let content = serde_json::to_string_pretty(profile)
            .context("Failed to serialize profile")?;
        
        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write profile file: {:?}", path))?;
        
        Ok(())
    }
    
    /// Create default profiles
    fn create_default_profiles(&mut self) -> Result<()> {
        // High Quality Profile
        let high_quality = self.create_high_quality_profile();
        self.add_profile(high_quality)?;
        
        // Fast Extraction Profile
        let fast_extraction = self.create_fast_extraction_profile();
        self.add_profile(fast_extraction)?;
        
        // Unity Development Profile
        let unity_dev = self.create_unity_development_profile();
        self.add_profile(unity_dev)?;
        
        // Research Profile
        let research = self.create_research_profile();
        self.add_profile(research)?;
        
        // ANIM Only Profile
        let anim_only = self.create_anim_only_profile();
        self.add_profile(anim_only)?;
        
        // Set default profile
        self.default_profile = Some("high_quality".to_string());
        
        Ok(())
    }
    
    /// Create high quality extraction profile
    fn create_high_quality_profile(&self) -> ConfigurationProfile {
        let mut config = ExtractionConfig::default();
        
        // High quality settings
        config.quality_settings.png_compression_level = 1; // Low compression for quality
        config.quality_settings.jpeg_quality = 95; // High JPEG quality
        config.quality_settings.prefer_lossless = true;
        config.quality_settings.color_depth = ColorDepth::Bit32;
        
        // Enable all formats
        config.format_settings.enabled_formats = vec![
            FormatType::ANIM, FormatType::GRP, FormatType::PCX, FormatType::PNG, FormatType::JPEG
        ];
        
        // High quality format settings
        for format in &config.format_settings.enabled_formats {
            config.format_settings.format_quality.insert(*format, FormatQuality {
                high_quality: true,
                preserve_color_depth: true,
                preserve_transparency: true,
                extract_metadata: true,
            });
        }
        
        // Performance settings for quality over speed
        config.performance_settings.use_streaming_processing = false; // Load everything for quality
        config.performance_settings.use_lazy_loading = false;
        
        // Comprehensive metadata
        config.output_settings.metadata_options.generate_json = true;
        config.output_settings.metadata_options.include_animation_data = true;
        config.output_settings.metadata_options.include_database_info = true;
        
        ConfigurationProfile {
            name: "high_quality".to_string(),
            description: "High quality extraction with maximum fidelity and comprehensive metadata".to_string(),
            version: "1.0.0".to_string(),
            author: Some("CASC Extractor Team".to_string()),
            tags: vec!["quality".to_string(), "comprehensive".to_string(), "metadata".to_string()],
            inherits_from: None,
            config,
            metadata: ProfileMetadata::new(),
        }
    }
    
    /// Create fast extraction profile
    fn create_fast_extraction_profile(&self) -> ConfigurationProfile {
        let mut config = ExtractionConfig::default();
        
        // Fast settings
        config.quality_settings.png_compression_level = 6; // Balanced compression
        config.quality_settings.jpeg_quality = 75; // Moderate quality
        config.quality_settings.prefer_lossless = false;
        
        // Performance optimizations
        config.performance_settings.use_streaming_processing = true;
        config.performance_settings.use_memory_mapping = true;
        config.performance_settings.use_lazy_loading = true;
        config.performance_settings.enable_object_pooling = true;
        config.performance_settings.batch_size = 200; // Larger batches
        
        // Minimal metadata for speed
        config.output_settings.metadata_options.generate_json = false;
        config.output_settings.metadata_options.include_animation_data = false;
        config.output_settings.metadata_options.include_database_info = false;
        
        // Simplified feedback
        config.feedback_settings.user_feedback_options.show_file_progress = false;
        config.feedback_settings.user_feedback_options.show_format_details = false;
        
        ConfigurationProfile {
            name: "fast_extraction".to_string(),
            description: "Fast extraction optimized for speed over quality".to_string(),
            version: "1.0.0".to_string(),
            author: Some("CASC Extractor Team".to_string()),
            tags: vec!["speed".to_string(), "performance".to_string(), "minimal".to_string()],
            inherits_from: None,
            config,
            metadata: ProfileMetadata::new(),
        }
    }
    
    /// Create Unity development profile
    fn create_unity_development_profile(&self) -> ConfigurationProfile {
        let mut config = ExtractionConfig::default();
        
        // Unity-optimized settings
        config.output_settings.unity_settings.enabled = true;
        config.output_settings.unity_settings.pixels_per_unit = 100.0;
        config.output_settings.unity_settings.filter_mode = UnityFilterMode::Point; // Pixel perfect
        config.output_settings.unity_settings.wrap_mode = UnityWrapMode::Clamp;
        config.output_settings.unity_settings.compression_quality = 75;
        config.output_settings.unity_settings.generate_mipmaps = false;
        config.output_settings.unity_settings.generate_meta_files = true;
        
        // Unity-friendly organization
        config.output_settings.naming_convention = NamingConvention::Unity;
        config.output_settings.directory_structure = DirectoryStructure::ByRace;
        
        // Unity metadata
        config.output_settings.metadata_options.generate_unity_meta = true;
        config.output_settings.metadata_options.include_animation_data = true;
        
        ConfigurationProfile {
            name: "unity_development".to_string(),
            description: "Optimized for Unity game development with proper import settings".to_string(),
            version: "1.0.0".to_string(),
            author: Some("CASC Extractor Team".to_string()),
            tags: vec!["unity".to_string(), "gamedev".to_string(), "import".to_string()],
            inherits_from: None,
            config,
            metadata: ProfileMetadata::new(),
        }
    }
    
    /// Create research profile
    fn create_research_profile(&self) -> ConfigurationProfile {
        let mut config = ExtractionConfig::default();
        
        // Research-oriented settings
        config.feedback_settings.collect_research_data = true;
        config.feedback_settings.collect_performance_metrics = true;
        config.feedback_settings.verbose_logging = true;
        config.feedback_settings.user_feedback_options.show_format_details = true;
        
        // Comprehensive metadata for research
        config.output_settings.metadata_options.generate_json = true;
        config.output_settings.metadata_options.include_animation_data = true;
        config.output_settings.metadata_options.include_database_info = true;
        config.output_settings.metadata_options.include_performance_metrics = true;
        config.output_settings.metadata_options.include_research_data = true;
        
        // Preserve original data
        config.quality_settings.color_depth = ColorDepth::Original;
        config.quality_settings.prefer_lossless = true;
        
        ConfigurationProfile {
            name: "research".to_string(),
            description: "Research-oriented extraction with comprehensive data collection".to_string(),
            version: "1.0.0".to_string(),
            author: Some("CASC Extractor Team".to_string()),
            tags: vec!["research".to_string(), "analysis".to_string(), "comprehensive".to_string()],
            inherits_from: None,
            config,
            metadata: ProfileMetadata::new(),
        }
    }
    
    /// Create ANIM-only profile
    fn create_anim_only_profile(&self) -> ConfigurationProfile {
        let mut config = ExtractionConfig::default();
        
        // ANIM format only
        config.format_settings.enabled_formats = vec![FormatType::ANIM];
        config.format_settings.extraction_mode = ExtractionMode::AnimOnly;
        
        // High quality for ANIM
        config.format_settings.format_quality.insert(FormatType::ANIM, FormatQuality {
            high_quality: true,
            preserve_color_depth: true,
            preserve_transparency: true,
            extract_metadata: true,
        });
        
        // HD resolution preference
        config.quality_settings.resolution_tier = ResolutionTier::HD;
        
        ConfigurationProfile {
            name: "anim_only".to_string(),
            description: "Extract only ANIM format sprites from StarCraft: Remastered".to_string(),
            version: "1.0.0".to_string(),
            author: Some("CASC Extractor Team".to_string()),
            tags: vec!["anim".to_string(), "remastered".to_string(), "selective".to_string()],
            inherits_from: None,
            config,
            metadata: ProfileMetadata::new(),
        }
    }
    
    /// Add a profile to the manager
    pub fn add_profile(&mut self, profile: ConfigurationProfile) -> Result<()> {
        // Validate profile
        profile.validate()?;
        
        // Check for name conflicts
        if self.profiles.contains_key(&profile.name) {
            return Err(anyhow::anyhow!("Profile with name '{}' already exists", profile.name));
        }
        
        // Add profile
        self.profiles.insert(profile.name.clone(), profile);
        
        Ok(())
    }
    
    /// Remove a profile
    pub fn remove_profile(&mut self, name: &str) -> Result<ConfigurationProfile> {
        let profile = self.profiles.remove(name)
            .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found", name))?;
        
        // Remove profile file
        let filename = format!("{}.json", name.replace(' ', "_").to_lowercase());
        let path = self.profile_directory.join(filename);
        if path.exists() {
            std::fs::remove_file(&path)
                .with_context(|| format!("Failed to remove profile file: {:?}", path))?;
        }
        
        // Update default if necessary
        if self.default_profile.as_deref() == Some(name) {
            self.default_profile = None;
        }
        
        Ok(profile)
    }
    
    /// Get a profile by name
    pub fn get_profile(&self, name: &str) -> Option<&ConfigurationProfile> {
        self.profiles.get(name)
    }
    
    /// Get a mutable profile by name
    pub fn get_profile_mut(&mut self, name: &str) -> Option<&mut ConfigurationProfile> {
        self.profiles.get_mut(name)
    }
    
    /// List all profile names
    pub fn list_profiles(&self) -> Vec<&String> {
        self.profiles.keys().collect()
    }
    
    /// Get profiles by tag
    pub fn get_profiles_by_tag(&self, tag: &str) -> Vec<&ConfigurationProfile> {
        self.profiles.values()
            .filter(|profile| profile.tags.contains(&tag.to_string()))
            .collect()
    }
    
    /// Apply a profile and return the resolved configuration
    pub fn apply_profile(&mut self, name: &str) -> Result<ExtractionConfig> {
        let _profile = self.get_profile(name)
            .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found", name))?;
        
        // Update usage statistics
        if let Some(profile) = self.get_profile_mut(name) {
            profile.metadata.usage_count += 1;
            profile.metadata.last_used_at = Some(chrono::Utc::now());
        }
        
        // Resolve inheritance
        self.resolve_profile_inheritance(name)
    }
    
    /// Resolve profile inheritance and return final configuration
    fn resolve_profile_inheritance(&self, name: &str) -> Result<ExtractionConfig> {
        let mut visited = std::collections::HashSet::new();
        self.resolve_profile_recursive(name, &mut visited)
    }
    
    /// Recursively resolve profile inheritance
    fn resolve_profile_recursive(&self, name: &str, visited: &mut std::collections::HashSet<String>) -> Result<ExtractionConfig> {
        // Check for circular inheritance
        if visited.contains(name) {
            return Err(anyhow::anyhow!("Circular inheritance detected in profile '{}'", name));
        }
        visited.insert(name.to_string());
        
        let profile = self.get_profile(name)
            .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found", name))?;
        
        let mut config = if let Some(base_name) = &profile.inherits_from {
            // Recursively resolve base profile
            self.resolve_profile_recursive(base_name, visited)?
        } else {
            // Start with default configuration
            ExtractionConfig::default()
        };
        
        // Merge current profile settings
        config.merge_with(&profile.config);
        
        visited.remove(name);
        Ok(config)
    }
    
    /// Set default profile
    pub fn set_default_profile(&mut self, name: Option<String>) -> Result<()> {
        if let Some(ref name) = name {
            if !self.profiles.contains_key(name) {
                return Err(anyhow::anyhow!("Profile '{}' not found", name));
            }
        }
        
        self.default_profile = name;
        Ok(())
    }
    
    /// Get default profile configuration
    pub fn get_default_config(&mut self) -> Result<ExtractionConfig> {
        if let Some(ref default_name) = self.default_profile.clone() {
            self.apply_profile(default_name)
        } else {
            Ok(ExtractionConfig::default())
        }
    }
}

impl ConfigurationProfile {
    /// Create a new profile
    pub fn new(name: String, description: String, config: ExtractionConfig) -> Self {
        Self {
            name,
            description,
            version: "1.0.0".to_string(),
            author: None,
            tags: Vec::new(),
            inherits_from: None,
            config,
            metadata: ProfileMetadata::new(),
        }
    }
    
    /// Validate the profile
    pub fn validate(&self) -> Result<()> {
        // Validate name
        if self.name.is_empty() {
            return Err(anyhow::anyhow!("Profile name cannot be empty"));
        }
        
        // Validate configuration
        self.config.validate()
            .context("Invalid profile configuration")?;
        
        Ok(())
    }
    
    /// Update profile metadata
    pub fn update_metadata(&mut self) {
        self.metadata.modified_at = chrono::Utc::now();
    }
    
    /// Set profile rating
    pub fn set_rating(&mut self, rating: u8) -> Result<()> {
        if rating > 5 {
            return Err(anyhow::anyhow!("Rating must be 1-5, got: {}", rating));
        }
        
        self.metadata.rating = Some(rating);
        self.update_metadata();
        Ok(())
    }
    
    /// Add a tag
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.update_metadata();
        }
    }
    
    /// Remove a tag
    pub fn remove_tag(&mut self, tag: &str) {
        self.tags.retain(|t| t != tag);
        self.update_metadata();
    }
}

impl ProfileMetadata {
    /// Create new metadata
    pub fn new() -> Self {
        let now = chrono::Utc::now();
        Self {
            created_at: now,
            modified_at: now,
            usage_count: 0,
            last_used_at: None,
            rating: None,
            notes: None,
        }
    }
}

impl Default for ProfileMetadata {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_profile_manager_creation() {
        let manager = ConfigurationProfileManager::new();
        assert!(manager.profiles.is_empty());
        assert!(manager.default_profile.is_none());
    }
    
    #[test]
    fn test_default_profiles_creation() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = ConfigurationProfileManager::with_profile_directory(temp_dir.path());
        
        manager.create_default_profiles().unwrap();
        
        assert!(!manager.profiles.is_empty());
        assert!(manager.get_profile("high_quality").is_some());
        assert!(manager.get_profile("fast_extraction").is_some());
        assert!(manager.get_profile("unity_development").is_some());
        assert!(manager.get_profile("research").is_some());
        assert!(manager.get_profile("anim_only").is_some());
    }
    
    #[test]
    fn test_profile_inheritance() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = ConfigurationProfileManager::with_profile_directory(temp_dir.path());
        
        // Create base profile
        let base_config = ExtractionConfig::default();
        let base_profile = ConfigurationProfile::new(
            "base".to_string(),
            "Base profile".to_string(),
            base_config,
        );
        manager.add_profile(base_profile).unwrap();
        
        // Create derived profile
        let mut derived_config = ExtractionConfig::default();
        derived_config.quality_settings.png_compression_level = 9;
        let mut derived_profile = ConfigurationProfile::new(
            "derived".to_string(),
            "Derived profile".to_string(),
            derived_config,
        );
        derived_profile.inherits_from = Some("base".to_string());
        manager.add_profile(derived_profile).unwrap();
        
        // Apply derived profile
        let resolved_config = manager.apply_profile("derived").unwrap();
        assert_eq!(resolved_config.quality_settings.png_compression_level, 9);
    }
    
    #[test]
    fn test_circular_inheritance_detection() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = ConfigurationProfileManager::with_profile_directory(temp_dir.path());
        
        // Create profiles with circular inheritance
        let mut profile_a = ConfigurationProfile::new(
            "a".to_string(),
            "Profile A".to_string(),
            ExtractionConfig::default(),
        );
        profile_a.inherits_from = Some("b".to_string());
        
        let mut profile_b = ConfigurationProfile::new(
            "b".to_string(),
            "Profile B".to_string(),
            ExtractionConfig::default(),
        );
        profile_b.inherits_from = Some("a".to_string());
        
        manager.add_profile(profile_a).unwrap();
        manager.add_profile(profile_b).unwrap();
        
        // Should detect circular inheritance
        let result = manager.apply_profile("a");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Circular inheritance"));
    }
    
    #[test]
    fn test_profile_tags() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = ConfigurationProfileManager::with_profile_directory(temp_dir.path());
        
        let mut profile = ConfigurationProfile::new(
            "test".to_string(),
            "Test profile".to_string(),
            ExtractionConfig::default(),
        );
        profile.add_tag("quality".to_string());
        profile.add_tag("unity".to_string());
        
        manager.add_profile(profile).unwrap();
        
        let quality_profiles = manager.get_profiles_by_tag("quality");
        assert_eq!(quality_profiles.len(), 1);
        assert_eq!(quality_profiles[0].name, "test");
        
        let unity_profiles = manager.get_profiles_by_tag("unity");
        assert_eq!(unity_profiles.len(), 1);
        assert_eq!(unity_profiles[0].name, "test");
    }
}