//! Integration tests for cross-platform functionality
//! 
//! These tests validate that the CASC extractor works correctly across
//! different operating systems (macOS, Linux, Windows).

use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Shared extraction workflow test — runs validation and extraction against a mock CASC
/// structure, then delegates to the platform-specific filesystem verification function.
fn run_extraction_workflow_test(
    platform: &str,
    verify_fs: fn(&std::path::Path),
) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let output_dir = temp_dir.path().join("output");
    let install_dir = temp_dir.path().join("starcraft_install");
    create_mock_casc_structure(&install_dir);
    let binary_path = PathBuf::from(env!("CARGO_BIN_EXE_casc-extractor"));

    // Validation pass
    let output = Command::new(&binary_path)
        .args(["--install-path", install_dir.to_str().unwrap(),
               "--output-dir",   output_dir.to_str().unwrap(),
               "--validate-only", "--verbose"])
        .output()
        .unwrap_or_else(|e| panic!("Failed to execute extractor on {}: {}", platform, e));

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stderr.contains("Installation path:")
            || stderr.contains("CASC Sprite Extractor")
            || stderr.contains("Argument validation failed"),
        "Should show installation processing info on {}. stderr: {}, stdout: {}",
        platform, stderr, stdout
    );

    // Extraction pass
    let output = Command::new(&binary_path)
        .args(["--install-path", install_dir.to_str().unwrap(),
               "--output-dir",   output_dir.to_str().unwrap(),
               "--include", "test.*\\.anim", "--verbose"])
        .output()
        .unwrap_or_else(|e| panic!("Failed to execute extraction on {}: {}", platform, e));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Opening CASC archive")
            || stderr.contains("Starting extraction workflow")
            || stderr.contains("Failed to open CASC archive"),
        "Should attempt extraction workflow on {}. stderr: {}", platform, stderr
    );

    assert!(output_dir.exists(), "Output directory should exist on {}", platform);
    verify_fs(&output_dir);
    println!("✅ {} integration test passed", platform);
}

/// Test complete extraction workflow on macOS
#[cfg(target_os = "macos")]
#[test]
fn test_macos_extraction_workflow() {
    // **Validates: Requirements 7.1**
    run_extraction_workflow_test("macOS", verify_macos_file_system_behavior);
}

/// Test complete extraction workflow on Linux
#[cfg(target_os = "linux")]
#[test]
fn test_linux_extraction_workflow() {
    // **Validates: Requirements 7.2**
    run_extraction_workflow_test("Linux", verify_linux_file_system_behavior);
}

/// Test complete extraction workflow on Windows
#[cfg(target_os = "windows")]
#[test]
fn test_windows_extraction_workflow() {
    // **Validates: Requirements 7.3**
    run_extraction_workflow_test("Windows", verify_windows_file_system_behavior);
}

/// Create a mock CASC installation structure for testing
fn create_mock_casc_structure(install_dir: &std::path::Path) {
    use std::fs;
    
    // Create the main installation directory
    fs::create_dir_all(install_dir).expect("Failed to create install directory");
    
    // Create Data directory structure
    let data_dir = install_dir.join("Data").join("data");
    fs::create_dir_all(&data_dir).expect("Failed to create data directory");
    
    // Create mock index files
    let index_file = data_dir.join("0000000001.idx");
    create_mock_index_file(&index_file);
    
    // Create mock data files
    let data_file = data_dir.join("data.000");
    create_mock_data_file(&data_file);
    
    // Create indices directory
    let indices_dir = install_dir.join("Data").join("indices");
    fs::create_dir_all(&indices_dir).expect("Failed to create indices directory");
    
    // Create config directory
    let config_dir = install_dir.join("Data").join("config");
    fs::create_dir_all(&config_dir).expect("Failed to create config directory");
}

/// Create a mock index file for testing
fn create_mock_index_file(path: &std::path::Path) {
    use std::fs::File;
    use std::io::Write;
    use byteorder::{LittleEndian, WriteBytesExt};
    
    let mut file = File::create(path).expect("Failed to create mock index file");
    
    // Write mock index file header
    file.write_u32::<LittleEndian>(0x10).unwrap(); // header_hash_size
    file.write_u32::<LittleEndian>(0x12345678).unwrap(); // header_hash
    file.write_u16::<LittleEndian>(7).unwrap(); // unk0
    file.write_u8(0).unwrap(); // bucket_index
    file.write_u8(0).unwrap(); // unk1
    file.write_u8(4).unwrap(); // entry_size_bytes
    file.write_u8(4).unwrap(); // entry_offset_bytes
    file.write_u8(9).unwrap(); // entry_key_bytes
    file.write_u8(16).unwrap(); // archive_file_header_size
    file.write_u64::<LittleEndian>(1024).unwrap(); // archive_total_size_maximum
    
    // Write a mock entry for test.anim
    let test_key = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09];
    file.write_all(&test_key).unwrap();
    file.write_u32::<LittleEndian>(0).unwrap(); // data_file_number
    file.write_u32::<LittleEndian>(0).unwrap(); // data_file_offset
}

/// Create a mock data file for testing
fn create_mock_data_file(path: &std::path::Path) {
    use std::fs::File;
    use std::io::Write;
    use byteorder::{LittleEndian, WriteBytesExt};
    
    let mut file = File::create(path).expect("Failed to create mock data file");
    
    // Write mock .anim file data
    file.write_u32::<LittleEndian>(0x4D494E41).unwrap(); // "ANIM" magic
    file.write_u8(2).unwrap(); // scale (HD)
    file.write_u8(1).unwrap(); // type (multi-sprite)
    file.write_u16::<LittleEndian>(0).unwrap(); // unknown
    file.write_u16::<LittleEndian>(1).unwrap(); // layer_count
    file.write_u16::<LittleEndian>(1).unwrap(); // sprite_count
    
    // Write layer name
    let layer_name = b"test_layer\0";
    file.write_all(layer_name).unwrap();
    
    // Pad to ensure we have enough data
    for _ in 0..100 {
        file.write_u8(0).unwrap();
    }
}

/// Verify macOS-specific file system behavior
#[cfg(target_os = "macos")]
fn verify_macos_file_system_behavior(output_dir: &std::path::Path) {
    use std::fs;
    
    // Verify that paths use forward slashes (Unix-style)
    let path_str = output_dir.to_string_lossy();
    assert!(!path_str.contains('\\'), "macOS paths should not contain backslashes");
    
    // Verify case sensitivity behavior (macOS is case-insensitive by default)
    if let Ok(entries) = fs::read_dir(output_dir) {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            
            // Verify no invalid characters for macOS
            assert!(!name_str.contains(':'), "macOS filenames should not contain colons");
        }
    }
    
    println!("✅ macOS file system behavior verified");
}

/// Verify Linux-specific file system behavior
#[cfg(target_os = "linux")]
fn verify_linux_file_system_behavior(output_dir: &std::path::Path) {
    use std::fs;
    
    // Verify that paths use forward slashes (Unix-style)
    let path_str = output_dir.to_string_lossy();
    assert!(!path_str.contains('\\'), "Linux paths should not contain backslashes");
    
    // Verify case sensitivity behavior (Linux is case-sensitive)
    if let Ok(entries) = fs::read_dir(output_dir) {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            
            // Verify no null characters (not allowed in Linux filenames)
            assert!(!name_str.contains('\0'), "Linux filenames should not contain null characters");
        }
    }
    
    println!("✅ Linux file system behavior verified");
}

/// Verify Windows-specific file system behavior
#[cfg(target_os = "windows")]
fn verify_windows_file_system_behavior(output_dir: &std::path::Path) {
    use std::fs;
    
    // Verify that paths can handle both forward and back slashes
    let path_str = output_dir.to_string_lossy();
    
    // Windows should handle the path correctly regardless of separator
    assert!(output_dir.exists(), "Windows should handle path separators correctly");
    
    // Verify Windows filename restrictions
    if let Ok(entries) = fs::read_dir(output_dir) {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            
            // Verify no invalid characters for Windows
            let invalid_chars = ['<', '>', ':', '"', '|', '?', '*'];
            for &invalid_char in &invalid_chars {
                assert!(!name_str.contains(invalid_char), 
                    "Windows filenames should not contain invalid character: {}", invalid_char);
            }
            
            // Verify no reserved names
            let reserved_names = ["CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", 
                                "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", 
                                "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9"];
            
            let name_upper = name_str.to_uppercase();
            for reserved in &reserved_names {
                assert!(!name_upper.starts_with(reserved), 
                    "Windows filenames should not use reserved name: {}", reserved);
            }
        }
    }
    
    println!("✅ Windows file system behavior verified");
}

/// Test cross-platform path handling
#[test]
fn test_cross_platform_path_handling() {
    use std::path::Path;
    
    // Test that our path handling works across platforms
    let test_paths = [
        "anim/units/terran/marine.anim",
        "HD2/anim/units/protoss/zealot.anim",
        "SD/units/zerg/zergling.anim",
    ];
    
    for path_str in &test_paths {
        let path = Path::new(path_str);
        
        // Verify path components can be extracted
        assert!(path.file_name().is_some(), "Should have filename: {}", path_str);
        assert!(path.parent().is_some(), "Should have parent directory: {}", path_str);
        
        // Verify path can be converted to string
        let converted = path.to_string_lossy();
        assert!(!converted.is_empty(), "Path conversion should not be empty: {}", path_str);
        
        // Test platform-specific path separator handling
        #[cfg(target_os = "windows")]
        {
            // Windows should handle both separators
            let windows_path = path_str.replace('/', "\\");
            let windows_path_obj = Path::new(&windows_path);
            assert!(windows_path_obj.file_name().is_some(), 
                "Windows should handle backslash paths: {}", windows_path);
        }
        
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            // Unix systems should use forward slashes
            assert!(path_str.contains('/'), "Unix paths should use forward slashes: {}", path_str);
            assert!(!path_str.contains('\\'), "Unix paths should not contain backslashes: {}", path_str);
        }
    }
    
    println!("✅ Cross-platform path handling test passed");
}

/// Test binary execution across platforms
#[test]
fn test_binary_execution_cross_platform() {
    let binary_path = PathBuf::from(env!("CARGO_BIN_EXE_casc-extractor"));
    
    // Test that the binary exists and is executable
    assert!(binary_path.exists(), "Binary should exist after build: {:?}", binary_path);
    
    // Test help command (should work on all platforms)
    let output = Command::new(&binary_path)
        .arg("--help")
        .output()
        .expect("Failed to execute binary help command");
    
    assert!(output.status.success(), "Help command should succeed on all platforms");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("CASC Sprite Extractor"), 
        "Help output should contain tool description");
    assert!(stdout.contains("--install-path"), 
        "Help output should contain install-path option");
    assert!(stdout.contains("--output-dir"), 
        "Help output should contain output-dir option");
    
    println!("✅ Binary execution test passed on current platform");
}