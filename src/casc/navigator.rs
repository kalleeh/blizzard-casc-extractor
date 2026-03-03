/// CASC Navigator System
///
/// This module provides functionality for automatically detecting StarCraft installations
/// and navigating CASC directory structures with transparent MPQ/CASC file system handling.
use std::path::{Path, PathBuf};
use std::fs;
use thiserror::Error;
use log::{info, warn, debug};

#[derive(Debug, Error)]
pub enum NavigatorError {
    #[error("No StarCraft installations found")]
    NoInstallationsFound,
    
    #[error("Installation not found at path: {0}")]
    InstallationNotFound(String),
    
    #[error("Invalid installation structure: {0}")]
    InvalidStructure(String),
    
    #[error("Access denied to installation: {0}")]
    AccessDenied(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Represents a detected StarCraft installation
#[derive(Debug, Clone)]
pub struct Installation {
    pub path: PathBuf,
    pub version: GameVersion,
    pub file_system_type: FileSystemType,
    pub display_name: String,
    pub is_valid: bool,
}

/// Game version detection
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameVersion {
    Classic,        // Original StarCraft (1998)
    Remastered,     // StarCraft: Remastered (2017+)
    Unknown,
}

/// File system type used by the installation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileSystemType {
    MPQ,    // Classic MPQ archives
    CASC,   // Modern CASC system
    Mixed,  // Both systems present
}

/// CASC Navigator for automatic installation detection and file system handling
pub struct CascNavigator {
    installations: Vec<Installation>,
    search_paths: Vec<PathBuf>,
}

impl CascNavigator {
    /// Create a new CASC Navigator
    pub fn new() -> Self {
        Self {
            installations: Vec::new(),
            search_paths: Self::get_default_search_paths(),
        }
    }
    
    /// Get default search paths for StarCraft installations
    /// Requirements 15.1: Automatic installation detection in standard locations
    fn get_default_search_paths() -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = Vec::new();
        
        #[cfg(target_os = "windows")]
        {
            // Windows standard installation paths
            paths.extend([
                // Program Files locations
                PathBuf::from("C:\\Program Files (x86)\\StarCraft"),
                PathBuf::from("C:\\Program Files\\StarCraft"),
                PathBuf::from("C:\\Program Files (x86)\\StarCraft II"),
                PathBuf::from("C:\\Program Files\\StarCraft II"),

                // Battle.net locations
                PathBuf::from("C:\\Program Files (x86)\\Battle.net\\Games\\StarCraft"),
                PathBuf::from("C:\\Program Files\\Battle.net\\Games\\StarCraft"),

                // Steam locations
                PathBuf::from("C:\\Program Files (x86)\\Steam\\steamapps\\common\\StarCraft"),
                PathBuf::from("C:\\Program Files\\Steam\\steamapps\\common\\StarCraft"),
            ]);
            // User-specific locations — only add if home/documents directories are available
            if let Some(home) = dirs::home_dir() {
                paths.push(home.join("Games\\StarCraft"));
            }
            if let Some(docs) = dirs::document_dir() {
                paths.push(docs.join("Games\\StarCraft"));
            }
        }
        
        #[cfg(target_os = "macos")]
        {
            // macOS standard installation paths
            paths.extend([
                // Applications
                PathBuf::from("/Applications/StarCraft"),
                PathBuf::from("/Applications/StarCraft II"),
            ]);
            // Home-relative locations — only add if home directory is available
            if let Some(home) = dirs::home_dir() {
                paths.push(home.join("Applications/StarCraft"));
                paths.push(home.join("Library/Application Support/Steam/steamapps/common/StarCraft"));
                paths.push(home.join("Applications/Battle.net/Games/StarCraft"));
            }
        }
        
        #[cfg(target_os = "linux")]
        {
            // Linux standard installation paths
            paths.extend([
                // System-wide installations
                PathBuf::from("/opt/starcraft"),
                PathBuf::from("/usr/local/games/starcraft"),
            ]);
            // Home-relative locations — only add if home directory is available
            if let Some(home) = dirs::home_dir() {
                paths.push(home.join("Games/StarCraft"));
                paths.push(home.join(".local/share/games/starcraft"));
                paths.push(home.join(".steam/steam/steamapps/common/StarCraft"));
                paths.push(home.join(".local/share/Steam/steamapps/common/StarCraft"));
                paths.push(home.join(".wine/drive_c/Program Files (x86)/StarCraft"));
                paths.push(home.join("Games/starcraft"));
            }
        }
        
        // Filter out empty paths and ensure they exist
        paths.into_iter()
            .filter(|p| !p.as_os_str().is_empty())
            .collect()
    }
    
    /// Automatically detect all StarCraft installations
    /// Requirements 15.1: Automatic detection in standard locations
    pub fn detect_installations(&mut self) -> Result<Vec<Installation>, NavigatorError> {
        info!("Scanning for StarCraft installations...");
        self.installations.clear();
        
        let mut found_count = 0;
        
        for search_path in &self.search_paths {
            debug!("Checking path: {:?}", search_path);
            
            if let Ok(installation) = self.analyze_path(search_path) {
                if installation.is_valid {
                    info!("Found valid installation: {} at {:?}", 
                          installation.display_name, installation.path);
                    self.installations.push(installation);
                    found_count += 1;
                } else {
                    debug!("Found invalid installation at {:?}", search_path);
                }
            }
        }
        
        // Also check environment variables for custom paths
        if let Ok(custom_path) = std::env::var("STARCRAFT_PATH") {
            let path = PathBuf::from(custom_path);
            if let Ok(installation) = self.analyze_path(&path) {
                if installation.is_valid {
                    info!("Found installation from STARCRAFT_PATH: {} at {:?}", 
                          installation.display_name, installation.path);
                    self.installations.push(installation);
                    found_count += 1;
                }
            }
        }
        
        if found_count == 0 {
            warn!("No valid StarCraft installations found in standard locations");
            return Err(NavigatorError::NoInstallationsFound);
        }
        
        info!("Found {} valid StarCraft installation(s)", found_count);
        Ok(self.installations.clone())
    }
    
    /// Analyze a specific path to determine if it's a valid StarCraft installation
    /// Requirements 15.2: CASC directory enumeration and file listing
    fn analyze_path(&self, path: &Path) -> Result<Installation, NavigatorError> {
        if !path.exists() {
            return Err(NavigatorError::InstallationNotFound(path.display().to_string()));
        }
        
        if !path.is_dir() {
            return Err(NavigatorError::InvalidStructure(
                format!("Path is not a directory: {}", path.display())
            ));
        }
        
        // Check for basic StarCraft directory structure
        let data_dir = path.join("Data");
        if !data_dir.exists() {
            return Err(NavigatorError::InvalidStructure(
                format!("Missing Data directory in: {}", path.display())
            ));
        }
        
        // Determine file system type and game version
        let file_system_type = self.detect_file_system_type(&data_dir)?;
        let version = self.detect_game_version(path, &file_system_type)?;
        
        // Validate the installation structure
        let is_valid = self.validate_installation_structure(path, &file_system_type)?;
        
        let display_name = self.generate_display_name(path, &version, &file_system_type);
        
        Ok(Installation {
            path: path.to_path_buf(),
            version,
            file_system_type,
            display_name,
            is_valid,
        })
    }
    
    /// Detect the file system type (MPQ vs CASC)
    /// Requirements 15.3: Transparent MPQ/CASC file system handling
    fn detect_file_system_type(&self, data_dir: &Path) -> Result<FileSystemType, NavigatorError> {
        let casc_data_dir = data_dir.join("data");
        let has_casc = casc_data_dir.exists() && self.has_casc_files(&casc_data_dir);
        
        let has_mpq = self.has_mpq_files(data_dir);
        
        match (has_casc, has_mpq) {
            (true, true) => Ok(FileSystemType::Mixed),
            (true, false) => Ok(FileSystemType::CASC),
            (false, true) => Ok(FileSystemType::MPQ),
            (false, false) => Err(NavigatorError::InvalidStructure(
                "No valid file system detected (neither CASC nor MPQ)".to_string()
            )),
        }
    }
    
    /// Scan `dir` for any entry whose file name satisfies `predicate`.
    fn has_files_matching(dir: &Path, predicate: impl Fn(&str) -> bool) -> bool {
        fs::read_dir(dir)
            .map(|entries| {
                entries
                    .filter_map(|entry| entry.ok())
                    .any(|entry| {
                        let name = entry.file_name();
                        predicate(&name.to_string_lossy())
                    })
            })
            .unwrap_or(false)
    }

    /// Check for CASC files in the data directory
    fn has_casc_files(&self, casc_data_dir: &Path) -> bool {
        if !casc_data_dir.is_dir() {
            return false;
        }

        // Look for CASC index files (*.idx)
        let has_index_files = Self::has_files_matching(casc_data_dir, |name| name.ends_with(".idx"));

        // Look for CASC data files (data.*)
        let has_data_files = Self::has_files_matching(casc_data_dir, |name| name.starts_with("data."));

        has_index_files && has_data_files
    }

    /// Check for MPQ files in the data directory
    fn has_mpq_files(&self, data_dir: &Path) -> bool {
        if !data_dir.is_dir() {
            return false;
        }

        // Look for MPQ files (*.mpq)
        Self::has_files_matching(data_dir, |name| {
            name.to_lowercase().ends_with(".mpq")
        })
    }
    
    /// Detect the game version based on directory structure and files
    fn detect_game_version(&self, install_path: &Path, file_system_type: &FileSystemType) -> Result<GameVersion, NavigatorError> {
        // Check for Remastered-specific indicators
        let has_remastered_indicators = [
            install_path.join("StarCraft.exe"),
            install_path.join("x86_64"),
            install_path.join("Data").join("data"), // CASC structure
        ].iter().any(|path| path.exists());
        
        // Check for Classic-specific indicators
        let has_classic_indicators = [
            install_path.join("StarCraft.exe"),
            install_path.join("Data").join("StarDat.mpq"),
            install_path.join("Data").join("BrooDat.mpq"),
        ].iter().any(|path| path.exists());
        
        match file_system_type {
            FileSystemType::CASC => Ok(GameVersion::Remastered),
            FileSystemType::MPQ => {
                if has_classic_indicators {
                    Ok(GameVersion::Classic)
                } else {
                    Ok(GameVersion::Unknown)
                }
            },
            FileSystemType::Mixed => {
                if has_remastered_indicators {
                    Ok(GameVersion::Remastered)
                } else if has_classic_indicators {
                    Ok(GameVersion::Classic)
                } else {
                    Ok(GameVersion::Unknown)
                }
            },
        }
    }
    
    /// Validate the installation structure
    fn validate_installation_structure(&self, install_path: &Path, file_system_type: &FileSystemType) -> Result<bool, NavigatorError> {
        match file_system_type {
            FileSystemType::CASC | FileSystemType::Mixed => {
                self.validate_casc_structure(install_path)
            },
            FileSystemType::MPQ => {
                self.validate_mpq_structure(install_path)
            },
        }
    }
    
    /// Validate CASC installation structure
    fn validate_casc_structure(&self, install_path: &Path) -> Result<bool, NavigatorError> {
        let data_dir = install_path.join("Data").join("data");
        
        if !data_dir.exists() {
            return Ok(false);
        }
        
        // Check for required CASC files
        let entries = fs::read_dir(&data_dir)?;
        let mut has_index_files = false;
        let mut has_data_files = false;
        
        for entry in entries {
            let entry = entry?;
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();
            
            if file_name_str.ends_with(".idx") {
                has_index_files = true;
            }
            
            if file_name_str.starts_with("data.") && !file_name_str.ends_with(".idx") {
                has_data_files = true;
            }
        }
        
        Ok(has_index_files && has_data_files)
    }
    
    /// Validate MPQ installation structure
    fn validate_mpq_structure(&self, install_path: &Path) -> Result<bool, NavigatorError> {
        let data_dir = install_path.join("Data");
        
        if !data_dir.exists() {
            return Ok(false);
        }
        
        // Check for essential MPQ files
        let required_mpqs = [
            "StarDat.mpq",
            "BrooDat.mpq",
        ];
        
        for mpq_name in &required_mpqs {
            let mpq_path = data_dir.join(mpq_name);
            if !mpq_path.exists() {
                debug!("Missing required MPQ file: {}", mpq_name);
                return Ok(false);
            }
        }
        
        Ok(true)
    }
    
    /// Generate a display name for the installation
    fn generate_display_name(&self, install_path: &Path, version: &GameVersion, file_system_type: &FileSystemType) -> String {
        let version_str = match version {
            GameVersion::Classic => "Classic",
            GameVersion::Remastered => "Remastered",
            GameVersion::Unknown => "Unknown",
        };
        
        let fs_str = match file_system_type {
            FileSystemType::MPQ => "MPQ",
            FileSystemType::CASC => "CASC",
            FileSystemType::Mixed => "Mixed",
        };
        
        let path_name = install_path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "StarCraft".to_string());
        
        format!("StarCraft {} ({}) - {}", version_str, fs_str, path_name)
    }
    
    /// Get all detected installations
    pub fn get_installations(&self) -> &[Installation] {
        &self.installations
    }
    
    /// Get the best installation (prioritizes Remastered over Classic)
    pub fn get_best_installation(&self) -> Option<&Installation> {
        // First, try to find a valid Remastered installation
        let remastered = self.installations
            .iter()
            .find(|inst| inst.is_valid && inst.version == GameVersion::Remastered);
        
        if let Some(inst) = remastered {
            return Some(inst);
        }
        
        // Fall back to any valid installation
        self.installations
            .iter()
            .find(|inst| inst.is_valid)
    }
    
    /// Add a custom search path
    pub fn add_search_path(&mut self, path: PathBuf) {
        if !self.search_paths.contains(&path) {
            self.search_paths.push(path);
        }
    }
    
    /// Enumerate all sprite-related files in an installation
    /// Requirements 15.2: CASC directory enumeration and file listing
    pub fn enumerate_sprite_files(&self, installation: &Installation) -> Result<Vec<FileInfo>, NavigatorError> {
        match installation.file_system_type {
            FileSystemType::CASC | FileSystemType::Mixed => {
                self.enumerate_casc_sprite_files(installation)
            },
            FileSystemType::MPQ => {
                self.enumerate_mpq_sprite_files(installation)
            },
        }
    }
    
    /// Enumerate sprite files from CASC system
    fn enumerate_casc_sprite_files(&self, installation: &Installation) -> Result<Vec<FileInfo>, NavigatorError> {
        // Use the existing CASC archive functionality
        let archive = crate::casc::CascArchive::open(&installation.path)
            .map_err(|e| NavigatorError::InvalidStructure(format!("Failed to open CASC archive: {}", e)))?;
        
        let files = archive.list_files_with_filter(Some("sprites"))
            .map_err(|e| NavigatorError::InvalidStructure(format!("Failed to list CASC files: {}", e)))?;
        
        Ok(files.into_iter().map(|f| FileInfo {
            name: f.name.clone(),
            path: PathBuf::from(&f.name),
            key: Some(f.key),
            size: None, // Size will be determined during extraction
            file_system_type: FileSystemType::CASC,
        }).collect())
    }
    
    /// Enumerate sprite files from MPQ system
    fn enumerate_mpq_sprite_files(&self, _installation: &Installation) -> Result<Vec<FileInfo>, NavigatorError> {
        // MPQ support would require additional implementation
        // For now, return empty list with a warning
        warn!("MPQ file enumeration not yet implemented");
        Ok(Vec::new())
    }
}

/// Information about a file in the archive
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub path: PathBuf,
    pub key: Option<[u8; 9]>, // CASC key, None for MPQ files
    pub size: Option<u64>,
    pub file_system_type: FileSystemType,
}

impl Default for CascNavigator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    fn create_mock_casc_installation(temp_dir: &TempDir) -> PathBuf {
        let install_path = temp_dir.path().join("StarCraft");
        let data_dir = install_path.join("Data");
        let casc_data_dir = data_dir.join("data");
        
        fs::create_dir_all(&casc_data_dir).unwrap();
        
        // Create mock CASC files
        fs::write(casc_data_dir.join("0000000001.idx"), b"mock index file").unwrap();
        fs::write(casc_data_dir.join("data.000"), b"mock data file").unwrap();
        
        // Create StarCraft.exe to indicate Remastered
        fs::write(install_path.join("StarCraft.exe"), b"mock executable").unwrap();
        
        install_path
    }
    
    fn create_mock_mpq_installation(temp_dir: &TempDir) -> PathBuf {
        let install_path = temp_dir.path().join("StarCraft_Classic");
        let data_dir = install_path.join("Data");
        
        fs::create_dir_all(&data_dir).unwrap();
        
        // Create mock MPQ files
        fs::write(data_dir.join("StarDat.mpq"), b"mock mpq file").unwrap();
        fs::write(data_dir.join("BrooDat.mpq"), b"mock mpq file").unwrap();
        
        // Create StarCraft.exe
        fs::write(install_path.join("StarCraft.exe"), b"mock executable").unwrap();
        
        install_path
    }
    
    #[test]
    fn test_detect_casc_installation() {
        let temp_dir = TempDir::new().unwrap();
        let install_path = create_mock_casc_installation(&temp_dir);
        
        let navigator = CascNavigator::new();
        let result = navigator.analyze_path(&install_path);
        
        assert!(result.is_ok());
        let installation = result.unwrap();
        assert_eq!(installation.version, GameVersion::Remastered);
        assert_eq!(installation.file_system_type, FileSystemType::CASC);
        assert!(installation.is_valid);
    }
    
    #[test]
    fn test_detect_mpq_installation() {
        let temp_dir = TempDir::new().unwrap();
        let install_path = create_mock_mpq_installation(&temp_dir);
        
        let navigator = CascNavigator::new();
        let result = navigator.analyze_path(&install_path);
        
        assert!(result.is_ok());
        let installation = result.unwrap();
        assert_eq!(installation.version, GameVersion::Classic);
        assert_eq!(installation.file_system_type, FileSystemType::MPQ);
        assert!(installation.is_valid);
    }
    
    #[test]
    fn test_invalid_installation_path() {
        let navigator = CascNavigator::new();
        let result = navigator.analyze_path(&PathBuf::from("/nonexistent/path"));
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), NavigatorError::InstallationNotFound(_)));
    }
    
    #[test]
    fn test_get_default_search_paths() {
        let paths = CascNavigator::get_default_search_paths();
        assert!(!paths.is_empty());
        
        // Verify that paths are platform-appropriate
        #[cfg(target_os = "windows")]
        {
            assert!(paths.iter().any(|p| p.to_string_lossy().contains("Program Files")));
        }
        
        #[cfg(target_os = "macos")]
        {
            assert!(paths.iter().any(|p| p.to_string_lossy().contains("Applications")));
        }
        
        #[cfg(target_os = "linux")]
        {
            assert!(paths.iter().any(|p| p.to_string_lossy().contains("Games") || p.to_string_lossy().contains("games")));
        }
    }
    
    #[test]
    fn test_file_system_detection() {
        let temp_dir = TempDir::new().unwrap();
        
        // Test CASC detection
        let casc_path = create_mock_casc_installation(&temp_dir);
        let navigator = CascNavigator::new();
        let casc_data_dir = casc_path.join("Data");
        let fs_type = navigator.detect_file_system_type(&casc_data_dir).unwrap();
        assert_eq!(fs_type, FileSystemType::CASC);
        
        // Test MPQ detection
        let mpq_path = create_mock_mpq_installation(&temp_dir);
        let mpq_data_dir = mpq_path.join("Data");
        let fs_type = navigator.detect_file_system_type(&mpq_data_dir).unwrap();
        assert_eq!(fs_type, FileSystemType::MPQ);
    }
    
    #[test]
    fn test_display_name_generation() {
        let navigator = CascNavigator::new();
        let path = PathBuf::from("/test/StarCraft");
        
        let name = navigator.generate_display_name(
            &path, 
            &GameVersion::Remastered, 
            &FileSystemType::CASC
        );
        
        assert_eq!(name, "StarCraft Remastered (CASC) - StarCraft");
    }
    
    #[test]
    fn test_get_best_installation() {
        let mut navigator = CascNavigator::new();
        
        // Add a Classic installation
        navigator.installations.push(Installation {
            path: PathBuf::from("/classic"),
            version: GameVersion::Classic,
            file_system_type: FileSystemType::MPQ,
            display_name: "Classic".to_string(),
            is_valid: true,
        });
        
        // Add a Remastered installation
        navigator.installations.push(Installation {
            path: PathBuf::from("/remastered"),
            version: GameVersion::Remastered,
            file_system_type: FileSystemType::CASC,
            display_name: "Remastered".to_string(),
            is_valid: true,
        });
        
        let best = navigator.get_best_installation().unwrap();
        assert_eq!(best.version, GameVersion::Remastered);
    }
}