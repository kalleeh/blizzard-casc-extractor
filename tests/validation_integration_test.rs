// Integration tests for the validation framework
//
// These tests demonstrate how to use the validation framework to ensure
// 100% byte-level accuracy and visual correctness of extracted sprites.

use casc_extractor::validation::{
    ReferenceValidator, ByteComparison, VisualComparison, RegressionTestSuite,
};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_byte_comparison_identical_files() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = temp_dir.path().join("file1.bin");
    let file2 = temp_dir.path().join("file2.bin");

    // Create identical files
    let data = b"test data for validation";
    File::create(&file1).unwrap().write_all(data).unwrap();
    File::create(&file2).unwrap().write_all(data).unwrap();

    // Compare files
    let result = ByteComparison::compare_files(&file1, &file2, false).unwrap();

    assert!(result.matches, "Files should match byte-for-byte");
    assert_eq!(result.hash1, result.hash2, "SHA256 hashes should match");
    assert_eq!(result.size1, result.size2, "File sizes should match");
    assert!(result.first_diff_offset.is_none(), "No differences should be found");
}

#[test]
fn test_byte_comparison_different_files() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = temp_dir.path().join("file1.bin");
    let file2 = temp_dir.path().join("file2.bin");

    // Create different files
    File::create(&file1).unwrap().write_all(b"data one").unwrap();
    File::create(&file2).unwrap().write_all(b"data two").unwrap();

    // Compare files
    let result = ByteComparison::compare_files(&file1, &file2, false).unwrap();

    assert!(!result.matches, "Files should not match");
    assert_ne!(result.hash1, result.hash2, "SHA256 hashes should differ");
    assert!(result.first_diff_offset.is_some(), "Difference offset should be found");
}

#[test]
fn test_byte_comparison_with_hex_dump() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = temp_dir.path().join("file1.bin");
    let file2 = temp_dir.path().join("file2.bin");

    // Create files with difference at specific offset
    File::create(&file1).unwrap().write_all(b"abcdefghijklmnop").unwrap();
    File::create(&file2).unwrap().write_all(b"abcdXfghijklmnop").unwrap();

    // Compare files with hex dump generation
    let result = ByteComparison::compare_files(&file1, &file2, true).unwrap();

    assert!(!result.matches, "Files should not match");
    assert_eq!(result.first_diff_offset, Some(4), "Difference should be at offset 4");
    assert!(result.hex_dump_path.is_some(), "Hex dump should be generated");
}

#[test]
fn test_sha256_calculation() {
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("test.bin");

    // Create a test file
    File::create(&file).unwrap().write_all(b"test data").unwrap();

    // Calculate SHA256
    let hash = ByteComparison::calculate_sha256(&file).unwrap();

    // Verify hash format (64 hex characters)
    assert_eq!(hash.len(), 64, "SHA256 hash should be 64 characters");
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()), "Hash should be hex");

    // Calculate again to verify consistency
    let hash2 = ByteComparison::calculate_sha256(&file).unwrap();
    assert_eq!(hash, hash2, "Hash should be consistent");
}

#[test]
fn test_visual_comparison_identical_images() {
    use image::RgbaImage;

    let temp_dir = TempDir::new().unwrap();
    let img1_path = temp_dir.path().join("img1.png");
    let img2_path = temp_dir.path().join("img2.png");

    // Create identical images
    let img = RgbaImage::from_pixel(64, 64, image::Rgba([128, 128, 128, 255]));
    img.save(&img1_path).unwrap();
    img.save(&img2_path).unwrap();

    // Compare images
    let result = VisualComparison::compare_images(&img1_path, &img2_path, false).unwrap();

    assert!(result.pixel_perfect_match, "Images should match pixel-for-pixel");
    assert_eq!(result.different_pixels, 0, "No pixels should differ");
    assert_eq!(result.perceptual_hash1, result.perceptual_hash2, "Perceptual hashes should match");
    assert_eq!(result.perceptual_distance, 0, "Perceptual distance should be 0");
}

#[test]
fn test_visual_comparison_different_images() {
    use image::RgbaImage;

    let temp_dir = TempDir::new().unwrap();
    let img1_path = temp_dir.path().join("img1.png");
    let img2_path = temp_dir.path().join("img2.png");

    // Create different images
    let img1 = RgbaImage::from_pixel(64, 64, image::Rgba([128, 128, 128, 255]));
    let img2 = RgbaImage::from_pixel(64, 64, image::Rgba([255, 0, 0, 255]));
    img1.save(&img1_path).unwrap();
    img2.save(&img2_path).unwrap();

    // Compare images
    let result = VisualComparison::compare_images(&img1_path, &img2_path, false).unwrap();

    assert!(!result.pixel_perfect_match, "Images should not match");
    assert_eq!(result.different_pixels, 64 * 64, "All pixels should differ");
    assert_eq!(result.difference_percentage, 100.0, "100% of pixels should differ");
}

#[test]
fn test_visual_comparison_with_diff_generation() {
    use image::RgbaImage;

    let temp_dir = TempDir::new().unwrap();
    let img1_path = temp_dir.path().join("img1.png");
    let img2_path = temp_dir.path().join("img2.png");

    // Create slightly different images
    let mut img1 = RgbaImage::from_pixel(64, 64, image::Rgba([128, 128, 128, 255]));
    let mut img2 = RgbaImage::from_pixel(64, 64, image::Rgba([128, 128, 128, 255]));
    
    // Make a few pixels different
    img2.put_pixel(10, 10, image::Rgba([255, 0, 0, 255]));
    img2.put_pixel(20, 20, image::Rgba([255, 0, 0, 255]));
    
    img1.save(&img1_path).unwrap();
    img2.save(&img2_path).unwrap();

    // Compare images with diff generation
    let result = VisualComparison::compare_images(&img1_path, &img2_path, true).unwrap();

    assert!(!result.pixel_perfect_match, "Images should not match");
    assert_eq!(result.different_pixels, 2, "2 pixels should differ");
    assert!(result.diff_image_path.is_some(), "Diff image should be generated");
    assert!(result.comparison_image_path.is_some(), "Comparison image should be generated");
}

#[test]
fn test_regression_suite_creation() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("regression_db.json");

    let suite = RegressionTestSuite::new(db_path).unwrap();
    assert_eq!(suite.count(), 0, "New suite should be empty");
}

#[test]
fn test_regression_suite_add_known_good() {
    use casc_extractor::validation::regression_suite::SpriteMetadata;

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("regression_db.json");
    let output_file = temp_dir.path().join("test_sprite.png");

    // Create a test sprite file
    File::create(&output_file).unwrap().write_all(b"test sprite data").unwrap();

    let mut suite = RegressionTestSuite::new(db_path).unwrap();

    let metadata = SpriteMetadata {
        width: 64,
        height: 64,
        frame_count: 1,
        format: "PNG".to_string(),
    };

    suite.add_known_good(
        "test_sprite".to_string(),
        PathBuf::from("source.casc"),
        output_file,
        metadata,
    ).unwrap();

    assert_eq!(suite.count(), 1, "Suite should have 1 known-good extraction");
    assert!(suite.has_baseline("test_sprite"), "Suite should have baseline for test_sprite");
}

#[test]
fn test_regression_suite_validation() {
    use casc_extractor::validation::regression_suite::SpriteMetadata;

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("regression_db.json");
    let output_file = temp_dir.path().join("test_sprite.png");

    // Create a test sprite file
    let data = b"test sprite data";
    File::create(&output_file).unwrap().write_all(data).unwrap();

    // Add to regression suite
    let mut suite = RegressionTestSuite::new(db_path).unwrap();
    let metadata = SpriteMetadata {
        width: 64,
        height: 64,
        frame_count: 1,
        format: "PNG".to_string(),
    };

    suite.add_known_good(
        "test_sprite".to_string(),
        PathBuf::from("source.casc"),
        output_file.clone(),
        metadata,
    ).unwrap();

    // Validate against the same file (should pass)
    let result = suite.validate_no_regression("test_sprite", &output_file).unwrap();
    assert!(result.passed, "Validation should pass for identical file");
    assert!(result.regression_details.is_none(), "No regression should be detected");
}

#[test]
fn test_regression_suite_detects_changes() {
    use casc_extractor::validation::regression_suite::SpriteMetadata;

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("regression_db.json");
    let original_file = temp_dir.path().join("original.png");
    let modified_file = temp_dir.path().join("modified.png");

    // Create original file
    File::create(&original_file).unwrap().write_all(b"original data").unwrap();

    // Add to regression suite
    let mut suite = RegressionTestSuite::new(db_path).unwrap();
    let metadata = SpriteMetadata {
        width: 64,
        height: 64,
        frame_count: 1,
        format: "PNG".to_string(),
    };

    suite.add_known_good(
        "test_sprite".to_string(),
        PathBuf::from("source.casc"),
        original_file,
        metadata,
    ).unwrap();

    // Create modified file
    File::create(&modified_file).unwrap().write_all(b"modified data").unwrap();

    // Validate against modified file (should fail)
    let result = suite.validate_no_regression("test_sprite", &modified_file).unwrap();
    assert!(!result.passed, "Validation should fail for modified file");
    assert!(result.regression_details.is_some(), "Regression should be detected");
}

#[test]
fn test_reference_validator_creation() {
    use casc_extractor::validation::reference_validator::ReferenceToolConfig;

    let config = ReferenceToolConfig::default();
    let validator = ReferenceValidator::new(config);

    // Validator should be created successfully
    // Actual validation requires reference tools to be installed
}

#[test]
fn test_byte_comparison_report_generation() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = temp_dir.path().join("file1.bin");
    let file2 = temp_dir.path().join("file2.bin");

    File::create(&file1).unwrap().write_all(b"test data").unwrap();
    File::create(&file2).unwrap().write_all(b"test data").unwrap();

    let result = ByteComparison::compare_files(&file1, &file2, false).unwrap();
    let report = ByteComparison::generate_report(&file1, &file2, &result);

    assert!(report.contains("Byte-Level Comparison Report"), "Report should have title");
    assert!(report.contains("FILES MATCH"), "Report should indicate match");
    assert!(report.contains(&result.hash1), "Report should include hash");
}

#[test]
fn test_visual_comparison_report_generation() {
    use image::RgbaImage;

    let temp_dir = TempDir::new().unwrap();
    let img1_path = temp_dir.path().join("img1.png");
    let img2_path = temp_dir.path().join("img2.png");

    let img = RgbaImage::from_pixel(64, 64, image::Rgba([128, 128, 128, 255]));
    img.save(&img1_path).unwrap();
    img.save(&img2_path).unwrap();

    let result = VisualComparison::compare_images(&img1_path, &img2_path, false).unwrap();
    let report = VisualComparison::generate_report(&img1_path, &img2_path, &result);

    assert!(report.contains("Visual Comparison Report"), "Report should have title");
    assert!(report.contains("IMAGES MATCH"), "Report should indicate match");
    assert!(report.contains("Perceptual Hash"), "Report should include perceptual hash");
}
