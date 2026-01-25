// Comprehensive tests for automated regression detection
//
// These tests validate that the regression detection system correctly identifies
// when previously working extractions break, and generates appropriate reports.

use casc_extractor::validation::{RegressionTestSuite, regression_suite::SpriteMetadata};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;
use image::GenericImageView;

#[test]
fn test_regression_detection_with_no_changes() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("regression_db.json");
    let output_dir = temp_dir.path().join("output");
    let report_path = temp_dir.path().join("regression_report.txt");
    
    std::fs::create_dir_all(&output_dir).unwrap();

    // Create a known-good extraction
    let sprite_file = output_dir.join("test_sprite.png");
    File::create(&sprite_file).unwrap().write_all(b"test sprite data").unwrap();

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
        sprite_file.clone(),
        metadata,
    ).unwrap();

    // Run regression detection (should pass since file hasn't changed)
    let result = suite.detect_regressions(&output_dir, &report_path);
    
    assert!(result.is_ok(), "Regression detection should pass when no changes");
    assert!(report_path.exists(), "Report should be generated");

    // Verify report content
    let report_content = std::fs::read_to_string(&report_path).unwrap();
    assert!(report_content.contains("ALL REGRESSION TESTS PASSED"), "Report should indicate success");
    assert!(report_content.contains("✅"), "Report should have success indicators");
}

#[test]
fn test_regression_detection_with_changes() {
    use image::RgbaImage;

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("regression_db.json");
    let output_dir = temp_dir.path().join("output");
    let report_path = temp_dir.path().join("regression_report.txt");
    
    std::fs::create_dir_all(&output_dir).unwrap();

    // Create original known-good extraction as PNG
    let original_file = temp_dir.path().join("original.png");
    let original_img = RgbaImage::from_pixel(32, 32, image::Rgba([128, 128, 128, 255]));
    original_img.save(&original_file).unwrap();

    let mut suite = RegressionTestSuite::new(db_path).unwrap();
    
    let metadata = SpriteMetadata {
        width: 32,
        height: 32,
        frame_count: 1,
        format: "PNG".to_string(),
    };

    suite.add_known_good(
        "test_sprite".to_string(),
        PathBuf::from("source.casc"),
        original_file,
        metadata,
    ).unwrap();

    // Create modified file in output directory
    let modified_file = output_dir.join("original.png");
    let modified_img = RgbaImage::from_pixel(32, 32, image::Rgba([255, 0, 0, 255]));
    modified_img.save(&modified_file).unwrap();

    // Run regression detection (should fail due to changes)
    let result = suite.detect_regressions(&output_dir, &report_path);
    
    assert!(result.is_err(), "Regression detection should fail when changes detected");
    assert!(report_path.exists(), "Report should be generated");

    // Verify report content
    let report_content = std::fs::read_to_string(&report_path).unwrap();
    assert!(report_content.contains("REGRESSION DETECTED"), "Report should indicate regression");
    assert!(report_content.contains("❌"), "Report should have failure indicators");
    assert!(report_content.contains("SHA256 hash mismatch"), "Report should show hash mismatch");
}

#[test]
fn test_regression_report_generation() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("regression_db.json");
    let output_dir = temp_dir.path().join("output");
    let report_path = temp_dir.path().join("regression_report.txt");
    
    std::fs::create_dir_all(&output_dir).unwrap();

    // Create multiple known-good extractions
    let mut suite = RegressionTestSuite::new(db_path).unwrap();
    
    let metadata = SpriteMetadata {
        width: 64,
        height: 64,
        frame_count: 1,
        format: "PNG".to_string(),
    };

    for i in 0..5 {
        let sprite_file = output_dir.join(format!("sprite_{}.png", i));
        File::create(&sprite_file).unwrap()
            .write_all(format!("sprite {} data", i).as_bytes())
            .unwrap();

        suite.add_known_good(
            format!("sprite_{}", i),
            PathBuf::from("source.casc"),
            sprite_file,
            metadata.clone(),
        ).unwrap();
    }

    // Run regression detection
    suite.detect_regressions(&output_dir, &report_path).unwrap();

    // Verify report structure
    let report_content = std::fs::read_to_string(&report_path).unwrap();
    
    assert!(report_content.contains("REGRESSION TEST SUITE REPORT"), "Report should have header");
    assert!(report_content.contains("SUMMARY:"), "Report should have summary section");
    assert!(report_content.contains("Total Tests:  5"), "Report should show correct test count");
    assert!(report_content.contains("Pass Rate:"), "Report should show pass rate");
    assert!(report_content.contains("DETAILED RESULTS:"), "Report should have detailed results");
}

#[test]
fn test_hash_mismatch_detection() {
    use image::RgbaImage;

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("regression_db.json");
    let output_dir = temp_dir.path().join("output");
    
    std::fs::create_dir_all(&output_dir).unwrap();

    // Create original PNG file
    let original_file = temp_dir.path().join("original.png");
    let original_img = RgbaImage::from_pixel(32, 32, image::Rgba([128, 128, 128, 255]));
    original_img.save(&original_file).unwrap();

    let mut suite = RegressionTestSuite::new(db_path).unwrap();
    
    let metadata = SpriteMetadata {
        width: 32,
        height: 32,
        frame_count: 1,
        format: "PNG".to_string(),
    };

    suite.add_known_good(
        "test_sprite".to_string(),
        PathBuf::from("source.casc"),
        original_file,
        metadata,
    ).unwrap();

    // Create file with different content
    let modified_file = output_dir.join("original.png");
    let modified_img = RgbaImage::from_pixel(32, 32, image::Rgba([255, 0, 0, 255]));
    modified_img.save(&modified_file).unwrap();

    // Validate - should detect hash mismatch
    let result = suite.validate_no_regression("test_sprite", &modified_file).unwrap();
    
    assert!(!result.passed, "Validation should fail for hash mismatch");
    assert!(result.regression_details.is_some(), "Should have regression details");
    
    let details = result.regression_details.unwrap();
    assert!(details.contains("SHA256 hash mismatch"), "Details should mention hash mismatch");
}

#[test]
fn test_comparison_image_generation() {
    use image::RgbaImage;

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("regression_db.json");
    let output_dir = temp_dir.path().join("output");
    let comparison_path = temp_dir.path().join("comparison.png");
    
    std::fs::create_dir_all(&output_dir).unwrap();

    // Create original image
    let original_file = temp_dir.path().join("original.png");
    let original_img = RgbaImage::from_pixel(64, 64, image::Rgba([128, 128, 128, 255]));
    original_img.save(&original_file).unwrap();

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

    // Create modified image
    let modified_file = output_dir.join("original.png");
    let modified_img = RgbaImage::from_pixel(64, 64, image::Rgba([255, 0, 0, 255]));
    modified_img.save(&modified_file).unwrap();

    // Generate comparison image
    suite.generate_comparison_image("test_sprite", &modified_file, &comparison_path).unwrap();

    assert!(comparison_path.exists(), "Comparison image should be generated");

    // Verify comparison image dimensions (should be side-by-side)
    let comparison_img = image::open(&comparison_path).unwrap();
    let (width, height) = comparison_img.dimensions();
    assert_eq!(width, 64 + 64 + 20, "Comparison should be side-by-side with gap");
    assert_eq!(height, 64, "Comparison height should match original");
}

#[test]
fn test_multiple_regressions_in_report() {
    use image::RgbaImage;

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("regression_db.json");
    let output_dir = temp_dir.path().join("output");
    let report_path = temp_dir.path().join("regression_report.txt");
    
    std::fs::create_dir_all(&output_dir).unwrap();

    let mut suite = RegressionTestSuite::new(db_path).unwrap();
    
    let metadata = SpriteMetadata {
        width: 32,
        height: 32,
        frame_count: 1,
        format: "PNG".to_string(),
    };

    // Create 3 sprites: 2 will pass, 1 will fail
    for i in 0..3 {
        let sprite_file = temp_dir.path().join(format!("sprite_{}.png", i));
        let img = RgbaImage::from_pixel(32, 32, image::Rgba([100 + i as u8 * 10, 100, 100, 255]));
        img.save(&sprite_file).unwrap();

        suite.add_known_good(
            format!("sprite_{}", i),
            PathBuf::from("source.casc"),
            sprite_file,
            metadata.clone(),
        ).unwrap();
    }

    // Create output files: modify sprite_1
    for i in 0..3 {
        let output_file = output_dir.join(format!("sprite_{}.png", i));
        let color = if i == 1 {
            image::Rgba([255, 0, 0, 255]) // Different color for sprite_1
        } else {
            image::Rgba([100 + i as u8 * 10, 100, 100, 255]) // Same as original
        };
        let img = RgbaImage::from_pixel(32, 32, color);
        img.save(&output_file).unwrap();
    }

    // Run regression detection
    let result = suite.detect_regressions(&output_dir, &report_path);
    
    assert!(result.is_err(), "Should detect regression");
    
    // Verify report shows correct counts
    let report_content = std::fs::read_to_string(&report_path).unwrap();
    assert!(report_content.contains("Total Tests:  3"), "Should show 3 total tests");
    assert!(report_content.contains("Passed:       2"), "Should show 2 passed");
    assert!(report_content.contains("Failed:       1"), "Should show 1 failed");
    assert!(report_content.contains("sprite_1"), "Should list the failed sprite");
}

#[test]
fn test_no_baseline_handling() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("regression_db.json");
    let output_file = temp_dir.path().join("test.png");
    
    File::create(&output_file).unwrap().write_all(b"test data").unwrap();

    let suite = RegressionTestSuite::new(db_path).unwrap();
    
    // Validate sprite with no baseline
    let result = suite.validate_no_regression("unknown_sprite", &output_file).unwrap();
    
    assert!(result.passed, "Should pass when no baseline exists");
    assert!(result.regression_details.is_some(), "Should note no baseline available");
    
    let details = result.regression_details.unwrap();
    assert!(details.contains("No baseline available"), "Should explain no baseline");
}

#[test]
fn test_regression_suite_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("regression_db.json");
    let sprite_file = temp_dir.path().join("test.png");
    
    File::create(&sprite_file).unwrap().write_all(b"test data").unwrap();

    // Create suite and add known-good
    {
        let mut suite = RegressionTestSuite::new(db_path.clone()).unwrap();
        
        let metadata = SpriteMetadata {
            width: 64,
            height: 64,
            frame_count: 1,
            format: "PNG".to_string(),
        };

        suite.add_known_good(
            "test_sprite".to_string(),
            PathBuf::from("source.casc"),
            sprite_file.clone(),
            metadata,
        ).unwrap();

        assert_eq!(suite.count(), 1);
    }

    // Load suite from disk
    {
        let suite = RegressionTestSuite::new(db_path).unwrap();
        assert_eq!(suite.count(), 1, "Suite should persist to disk");
        assert!(suite.has_baseline("test_sprite"), "Baseline should be loaded");
    }
}
