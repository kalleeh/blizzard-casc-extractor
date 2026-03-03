use anyhow::{Result, Context};
use std::collections::HashMap;
use std::path::Path;
use crate::casc::CascArchive;

/// Analyzes CASC file structure to understand sprite formats
pub struct FormatAnalyzer {
    archive: CascArchive,
}

impl FormatAnalyzer {
    pub fn new(archive: CascArchive) -> Self {
        Self { archive }
    }

    /// Analyze file patterns in CASC to understand sprite organization
    pub fn analyze_sprite_patterns(&self) -> Result<SpritePatternAnalysis> {
        log::info!("Analyzing CASC file patterns for sprite data...");
        
        let mut analysis = SpritePatternAnalysis::new();
        
        // Get all files from CASC
        let files = self.archive.list_all_files()
            .context("Failed to list CASC files")?;
        
        log::info!("Found {} files in CASC archive", files.len());
        
        // Analyze file patterns
        for file_info in files {
            self.analyze_file_pattern(&file_info, &mut analysis);
        }
        
        // Log analysis results
        log::info!("=== SPRITE PATTERN ANALYSIS ===");
        log::info!("ANIM files: {}", analysis.anim_files.len());
        log::info!("GRP files: {}", analysis.grp_files.len());
        log::info!("DDS files: {}", analysis.dds_files.len());
        log::info!("PNG files: {}", analysis.png_files.len());
        log::info!("JPEG files: {}", analysis.jpeg_files.len());
        log::info!("Unknown sprite files: {}", analysis.unknown_sprite_files.len());
        
        // Analyze directory structure
        self.analyze_directory_structure(&analysis);
        
        Ok(analysis)
    }
    
    fn analyze_file_pattern(&self, file_info: &crate::casc::FileInfo, analysis: &mut SpritePatternAnalysis) {
        let path = &file_info.name;
        let path_lower = path.to_lowercase();
        
        // Check for known sprite file extensions
        if path_lower.ends_with(".anim") {
            analysis.anim_files.push(file_info.clone());
            self.analyze_anim_path(path, analysis);
        } else if path_lower.ends_with(".grp") {
            analysis.grp_files.push(file_info.clone());
        } else if path_lower.ends_with(".dds") {
            analysis.dds_files.push(file_info.clone());
        } else if path_lower.ends_with(".dds.grp") {
            analysis.dds_grp_files.push(file_info.clone());
        } else if path_lower.ends_with(".png") {
            analysis.png_files.push(file_info.clone());
        } else if path_lower.ends_with(".jpg") || path_lower.ends_with(".jpeg") {
            analysis.jpeg_files.push(file_info.clone());
        } else if self.is_potential_sprite_file(path) {
            analysis.unknown_sprite_files.push(file_info.clone());
        }
        
        // Analyze directory patterns
        if let Some(parent) = Path::new(path).parent() {
            let parent_str = parent.to_string_lossy().to_string();
            *analysis.directory_counts.entry(parent_str).or_insert(0) += 1;
        }
    }
    
    fn analyze_anim_path(&self, path: &str, analysis: &mut SpritePatternAnalysis) {
        // Check for resolution patterns based on Animosity's expected structure
        if path.contains("/anim/main_") {
            if path.contains("/HD2/") {
                analysis.hd2_anim_files += 1;
            } else {
                analysis.hd_anim_files += 1;
            }
        } else if path.contains("mainSD.anim") || path.contains("/SD/") {
            analysis.sd_anim_files += 1;
        }
    }
    
    fn is_potential_sprite_file(&self, path: &str) -> bool {
        let path_lower = path.to_lowercase();
        
        // Check for sprite-related directory patterns
        if path_lower.contains("sprite") || 
           path_lower.contains("unit") || 
           path_lower.contains("building") ||
           path_lower.contains("effect") ||
           path_lower.contains("portrait") {
            return true;
        }
        
        // Check for image-like file patterns without extensions
        if path_lower.contains("main_") && path_lower.len() > 10 {
            return true;
        }
        
        false
    }
    
    fn analyze_directory_structure(&self, analysis: &SpritePatternAnalysis) {
        log::info!("=== DIRECTORY STRUCTURE ANALYSIS ===");
        
        // Sort directories by file count
        let mut sorted_dirs: Vec<_> = analysis.directory_counts.iter().collect();
        sorted_dirs.sort_by(|a, b| b.1.cmp(a.1));
        
        // Show top directories
        for (dir, count) in sorted_dirs.iter().take(20) {
            log::info!("  {}: {} files", dir, count);
        }
        
        // Look for Animosity-expected patterns
        log::info!("=== EXPECTED PATTERN ANALYSIS ===");
        
        let expected_patterns = [
            "anim/main_",
            "HD2/anim/main_",
            "SD/mainSD.anim",
            "arr/images.dat",
            "arr/images.tbl",
            "unit/",
        ];
        
        for pattern in &expected_patterns {
            let matching_files: Vec<_> = analysis.all_files()
                .filter(|f| f.name.contains(pattern))
                .collect();
            
            log::info!("  Pattern '{}': {} matches", pattern, matching_files.len());
            
            if !matching_files.is_empty() {
                for file in matching_files.iter().take(5) {
                    log::info!("    - {}", file.name);
                }
                if matching_files.len() > 5 {
                    log::info!("    ... and {} more", matching_files.len() - 5);
                }
            }
        }
    }
    
    /// Extract sample files for format analysis
    pub fn extract_samples(&self, output_dir: &Path, analysis: &SpritePatternAnalysis) -> Result<()> {
        log::info!("Extracting sample files for format analysis...");
        
        std::fs::create_dir_all(output_dir)
            .context("Failed to create output directory")?;
        
        // Extract a few samples of each type
        self.extract_file_samples(&analysis.anim_files, output_dir, "anim", 3)?;
        self.extract_file_samples(&analysis.grp_files, output_dir, "grp", 3)?;
        self.extract_file_samples(&analysis.dds_files, output_dir, "dds", 3)?;
        self.extract_file_samples(&analysis.dds_grp_files, output_dir, "dds_grp", 3)?;
        self.extract_file_samples(&analysis.png_files, output_dir, "png", 3)?;
        self.extract_file_samples(&analysis.jpeg_files, output_dir, "jpeg", 3)?;
        
        // Extract any files matching expected patterns
        let expected_files = [
            "arr/images.dat",
            "arr/images.tbl",
        ];
        
        for expected in &expected_files {
            if let Some(file) = analysis.all_files().find(|f| f.name.contains(expected)) {
                self.extract_single_file(file, output_dir)?;
            }
        }
        
        log::info!("Sample extraction completed");
        Ok(())
    }
    
    fn extract_file_samples(
        &self, 
        files: &[crate::casc::FileInfo], 
        output_dir: &Path, 
        subdir: &str, 
        max_count: usize
    ) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }
        
        let sample_dir = output_dir.join(subdir);
        std::fs::create_dir_all(&sample_dir)
            .context("Failed to create sample directory")?;
        
        for (i, file) in files.iter().take(max_count).enumerate() {
            let filename = format!("sample_{:02}_{}", i, 
                Path::new(&file.name).file_name()
                    .unwrap_or_default()
                    .to_string_lossy());
            
            let output_path = sample_dir.join(filename);
            
            match self.archive.extract_file(&file.name, &output_path) {
                Ok(_) => log::info!("Extracted sample: {}", output_path.display()),
                Err(e) => log::warn!("Failed to extract {}: {}", file.name, e),
            }
        }
        
        Ok(())
    }
    
    fn extract_single_file(&self, file: &crate::casc::FileInfo, output_dir: &Path) -> Result<()> {
        let filename = Path::new(&file.name).file_name()
            .unwrap_or_default()
            .to_string_lossy();
        
        let output_path = output_dir.join(filename.as_ref());
        
        match self.archive.extract_file(&file.name, &output_path) {
            Ok(_) => {
                log::info!("Extracted expected file: {}", output_path.display());
                Ok(())
            }
            Err(e) => {
                log::warn!("Failed to extract {}: {}", file.name, e);
                Err(e.into())
            }
        }
    }
}

#[derive(Debug)]
pub struct SpritePatternAnalysis {
    pub anim_files: Vec<crate::casc::FileInfo>,
    pub grp_files: Vec<crate::casc::FileInfo>,
    pub dds_files: Vec<crate::casc::FileInfo>,
    pub dds_grp_files: Vec<crate::casc::FileInfo>,
    pub png_files: Vec<crate::casc::FileInfo>,
    pub jpeg_files: Vec<crate::casc::FileInfo>,
    pub unknown_sprite_files: Vec<crate::casc::FileInfo>,
    
    // Resolution-specific counts
    pub hd_anim_files: usize,
    pub hd2_anim_files: usize,
    pub sd_anim_files: usize,
    
    // Directory analysis
    pub directory_counts: HashMap<String, usize>,
}

impl SpritePatternAnalysis {
    fn new() -> Self {
        Self {
            anim_files: Vec::new(),
            grp_files: Vec::new(),
            dds_files: Vec::new(),
            dds_grp_files: Vec::new(),
            png_files: Vec::new(),
            jpeg_files: Vec::new(),
            unknown_sprite_files: Vec::new(),
            hd_anim_files: 0,
            hd2_anim_files: 0,
            sd_anim_files: 0,
            directory_counts: HashMap::new(),
        }
    }
    
    fn all_files(&self) -> impl Iterator<Item = &crate::casc::FileInfo> {
        self.anim_files.iter()
            .chain(self.grp_files.iter())
            .chain(self.dds_files.iter())
            .chain(self.dds_grp_files.iter())
            .chain(self.png_files.iter())
            .chain(self.jpeg_files.iter())
            .chain(self.unknown_sprite_files.iter())
    }
    
    pub fn total_sprite_files(&self) -> usize {
        self.anim_files.len() + 
        self.grp_files.len() + 
        self.dds_files.len() + 
        self.dds_grp_files.len() + 
        self.png_files.len() + 
        self.jpeg_files.len() + 
        self.unknown_sprite_files.len()
    }
    
    pub fn has_expected_structure(&self) -> bool {
        // Check if we have the expected Animosity structure
        self.anim_files.iter().any(|f| f.name.contains("main_")) ||
        self.grp_files.iter().any(|f| f.name.contains("unit/")) ||
        !self.dds_grp_files.is_empty()
    }
    
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str("# CASC Sprite Pattern Analysis Report\n\n");
        
        report.push_str("## File Type Summary\n");
        report.push_str(&format!("- ANIM files: {}\n", self.anim_files.len()));
        report.push_str(&format!("- GRP files: {}\n", self.grp_files.len()));
        report.push_str(&format!("- DDS files: {}\n", self.dds_files.len()));
        report.push_str(&format!("- DDS.GRP files: {}\n", self.dds_grp_files.len()));
        report.push_str(&format!("- PNG files: {}\n", self.png_files.len()));
        report.push_str(&format!("- JPEG files: {}\n", self.jpeg_files.len()));
        report.push_str(&format!("- Unknown sprite files: {}\n", self.unknown_sprite_files.len()));
        report.push_str(&format!("- **Total sprite files: {}**\n\n", self.total_sprite_files()));
        
        report.push_str("## Resolution Analysis\n");
        report.push_str(&format!("- HD ANIM files: {}\n", self.hd_anim_files));
        report.push_str(&format!("- HD2 ANIM files: {}\n", self.hd2_anim_files));
        report.push_str(&format!("- SD ANIM files: {}\n", self.sd_anim_files));
        
        report.push_str("\n## Structure Compatibility\n");
        report.push_str(&format!("- Has expected Animosity structure: {}\n", self.has_expected_structure()));
        
        report.push_str("\n## Top Directories\n");
        let mut sorted_dirs: Vec<_> = self.directory_counts.iter().collect();
        sorted_dirs.sort_by(|a, b| b.1.cmp(a.1));
        
        for (dir, count) in sorted_dirs.iter().take(10) {
            report.push_str(&format!("- {}: {} files\n", dir, count));
        }
        
        report
    }
}