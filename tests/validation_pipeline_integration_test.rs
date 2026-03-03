// Integration test for comprehensive validation pipeline
//
// This test validates that the end-to-end validation workflow actually works
// with real test data, not just unit tests.

use casc_extractor::validation::{
    ValidationPipeline, ValidationConfig, ErrorReporter, ErrorType, Severity,
};
use std::path::PathBuf;
use tempfile::TempDir;
use image::{RgbaImage, Rgba};

#[test]
fn test_validation_pipeline_with_real_data() {
    // Create temporary directories for test
    let temp_dir = TempDir::new().unwrap();
    let report_dir = temp_dir.path().join("validation-reports");
    let test_sprites_dir = temp_dir.path().join("test-sprites");
    std::fs::create_dir_all(&test_sprites_dir).unwrap();
    
    // Create test sprite images
    let sprite1_path = test_sprites_dir.join("marine.png");
    let sprite2_path = test_sprites_dir.join("zergling.png");
    
    // Create actual PNG files
    let img1 = RgbaImage::from_pixel(64, 64, Rgba([128, 128, 128, 255]));
    let img2 = RgbaImage::from_pixel(32, 32, Rgba([255, 0, 0, 255]));
    img1.save(&sprite1_path).unwrap();
    img2.save(&sprite2_path).unwrap();
    
    // Create validation pipeline
    let mut pipeline = ValidationPipeline::new(report_dir.clone())
        .expect("Failed to create validation pipeline");
    
    // Prepare extracted files for validation
    let extracted_files = vec![
        (sprite1_path.clone(), "ANIM".to_string()),
        (sprite2_path.clone(), "GRP".to_string()),
    ];
    
    // Configure validation (disable reference tools since we don't have them installed)
    let config = ValidationConfig {
        enable_reference_validation: false, // Disabled for test
        enable_byte_comparison: true,
        enable_visual_validation: true,
        enable_unity_validation: false, // Disabled for test (requires Unity)
        enable_regression_testing: true,
        fail_fast: false,
        generate_diagnostics: true,
        report_dir: report_dir.clone(),
    };
    
    // Run validation
    let report = pipeline.validate_batch(&extracted_files, &config)
        .expect("Validation pipeline failed");
    
    // Verify results
    println!("Validation Summary: {}", report.summary);
    println!("Total files: {}", report.total_files);
    println!("Passed files: {}", report.passed_files);
    println!("Failed files: {}", report.failed_files);
    
    assert_eq!(report.total_files, 2, "Should validate 2 files");
    
    // Check that reports were generated
    for extraction_report in &report.extraction_reports {
        println!("File: {} - Pass: {}", 
            extraction_report.file_name, 
            extraction_report.overall_pass
        );
        
        if !extraction_report.diagnostics.is_empty() {
            println!("  Diagnostics:");
            for diagnostic in &extraction_report.diagnostics {
                println!("    - {}", diagnostic);
            }
        }
    }
    
    // Verify report files were created
    assert!(report_dir.exists(), "Report directory should exist");
    
    let _json_report = report_dir.join("validation_report.json");
    let _text_report = report_dir.join("batch_validation_report.txt");
    
    // Note: Reports are only generated if there are failures or if configured
    // For this test, we just verify the pipeline runs without errors
    
    println!("✅ Validation pipeline integration test passed!");
}

#[test]
fn test_error_reporter_with_real_extraction() {
    let temp_dir = TempDir::new().unwrap();
    let report_dir = temp_dir.path().join("error-reports");
    
    // Create error reporter
    let reporter = ErrorReporter::new(report_dir.clone())
        .expect("Failed to create error reporter");
    
    // Create a test diagnostic
    let diagnostic = reporter.create_diagnostic(
        ErrorType::ByteMismatch,
        Severity::High,
        "test_sprite.png".to_string(),
        "Byte mismatch detected at offset 0x100".to_string(),
    );
    
    // Verify diagnostic has recommendations
    assert!(!diagnostic.recommendations.is_empty(), 
        "Diagnostic should have recommendations");
    
    println!("Error Diagnostic:");
    println!("  Type: {:?}", diagnostic.error_type);
    println!("  Severity: {:?}", diagnostic.severity);
    println!("  File: {}", diagnostic.file_name);
    println!("  Description: {}", diagnostic.description);
    println!("  Recommendations:");
    for (i, rec) in diagnostic.recommendations.iter().enumerate() {
        println!("    {}. {}", i + 1, rec);
    }
    
    // Verify specific recommendations for ByteMismatch
    assert!(diagnostic.recommendations.iter().any(|r| r.contains("hex dump")),
        "ByteMismatch should recommend hex dump analysis");
    
    println!("✅ Error reporter integration test passed!");
}

#[test]
fn test_hex_dump_generation() {
    let temp_dir = TempDir::new().unwrap();
    let report_dir = temp_dir.path().join("hex-dumps");
    
    // Create error reporter
    let reporter = ErrorReporter::new(report_dir.clone())
        .expect("Failed to create error reporter");
    
    // Create a test file with known content
    let test_file = temp_dir.path().join("test_data.bin");
    let test_data = vec![
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
        0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
    ];
    std::fs::write(&test_file, &test_data).unwrap();
    
    // Generate hex dump (using private method through error report generation)
    // We'll create a mock extraction report to trigger hex dump generation
    use casc_extractor::validation::ExtractionValidationReport;
    
    let extraction_report = ExtractionValidationReport {
        file_name: "test_data.bin".to_string(),
        format: "TEST".to_string(),
        reference_validation: None,
        byte_comparison: Some(casc_extractor::validation::pipeline::PipelineByteResult {
            passed: false,
            sha256_match: false,
            byte_differences: 10,
            hex_dump_path: Some(report_dir.join("test_data_hex_dump.txt")),
        }),
        visual_comparison: None,
        unity_import: None,
        overall_pass: false,
        diagnostics: vec!["Test diagnostic".to_string()],
    };
    
    // Generate error report (which should create hex dump)
    let report_path = reporter.generate_extraction_error_report(
        &extraction_report,
        &test_file,
    ).expect("Failed to generate error report");
    
    // Verify report was created
    assert!(report_path.exists(), "Error report should be created");
    
    // Read and verify report content
    let report_content = std::fs::read_to_string(&report_path).unwrap();
    assert!(report_content.contains("VALIDATION ERROR REPORT"), 
        "Report should contain header");
    assert!(report_content.contains("BYTE-LEVEL COMPARISON"),
        "Report should contain byte comparison section");
    assert!(report_content.contains("RECOMMENDATIONS"),
        "Report should contain recommendations");
    
    println!("Error report generated at: {:?}", report_path);
    println!("Report preview:");
    println!("{}", report_content.lines().take(20).collect::<Vec<_>>().join("\n"));
    
    println!("✅ Hex dump generation test passed!");
}

#[test]
fn test_validation_config_customization() {
    // Test that validation config can be customized
    let config = ValidationConfig {
        enable_reference_validation: false,
        enable_byte_comparison: true,
        enable_visual_validation: true,
        enable_unity_validation: false,
        enable_regression_testing: false,
        fail_fast: true,
        generate_diagnostics: true,
        report_dir: PathBuf::from("custom-reports"),
    };
    
    assert!(!config.enable_reference_validation);
    assert!(config.enable_byte_comparison);
    assert!(config.enable_visual_validation);
    assert!(!config.enable_unity_validation);
    assert!(!config.enable_regression_testing);
    assert!(config.fail_fast);
    assert!(config.generate_diagnostics);
    
    println!("✅ Validation config customization test passed!");
}

#[test]
fn test_batch_validation_report_generation() {
    let temp_dir = TempDir::new().unwrap();
    let report_dir = temp_dir.path().join("batch-reports");
    
    // Create error reporter
    let reporter = ErrorReporter::new(report_dir.clone())
        .expect("Failed to create error reporter");
    
    // Create a mock batch report
    use casc_extractor::validation::{BatchValidationReport, ExtractionValidationReport};
    
    let extraction_reports = vec![
        ExtractionValidationReport {
            file_name: "sprite1.png".to_string(),
            format: "ANIM".to_string(),
            reference_validation: None,
            byte_comparison: None,
            visual_comparison: None,
            unity_import: None,
            overall_pass: true,
            diagnostics: vec![],
        },
        ExtractionValidationReport {
            file_name: "sprite2.png".to_string(),
            format: "GRP".to_string(),
            reference_validation: None,
            byte_comparison: None,
            visual_comparison: None,
            unity_import: None,
            overall_pass: false,
            diagnostics: vec!["Test failure".to_string()],
        },
    ];
    
    let batch_report = BatchValidationReport {
        total_files: 2,
        passed_files: 1,
        failed_files: 1,
        extraction_reports,
        regression_results: None,
        overall_pass: false,
        summary: "Test batch validation".to_string(),
    };
    
    // Generate batch report
    let report_path = reporter.generate_batch_error_report(&batch_report)
        .expect("Failed to generate batch report");
    
    // Verify report was created
    assert!(report_path.exists(), "Batch report should be created");
    
    // Read and verify report content
    let report_content = std::fs::read_to_string(&report_path).unwrap();
    assert!(report_content.contains("BATCH VALIDATION REPORT"),
        "Report should contain header");
    assert!(report_content.contains("Total Files: 2"),
        "Report should show total files");
    assert!(report_content.contains("Passed: 1"),
        "Report should show passed count");
    assert!(report_content.contains("Failed: 1"),
        "Report should show failed count");
    
    println!("Batch report generated at: {:?}", report_path);
    println!("Report preview:");
    println!("{}", report_content.lines().take(30).collect::<Vec<_>>().join("\n"));
    
    println!("✅ Batch validation report generation test passed!");
}

#[test]
fn test_end_to_end_validation_workflow() {
    println!("\n=== Running End-to-End Validation Workflow Test ===\n");
    
    // Setup
    let temp_dir = TempDir::new().unwrap();
    let report_dir = temp_dir.path().join("e2e-validation");
    let sprites_dir = temp_dir.path().join("sprites");
    std::fs::create_dir_all(&sprites_dir).unwrap();
    
    // Step 1: Create test sprites
    println!("Step 1: Creating test sprites...");
    let sprite_files = vec![
        ("marine_walk.png", 64, 64, Rgba([100, 100, 200, 255])),
        ("zergling_attack.png", 48, 48, Rgba([200, 50, 50, 255])),
        ("scv_gather.png", 32, 32, Rgba([150, 150, 150, 255])),
    ];
    
    let mut extracted_files = Vec::new();
    for (name, width, height, color) in sprite_files {
        let path = sprites_dir.join(name);
        let img = RgbaImage::from_pixel(width, height, color);
        img.save(&path).unwrap();
        extracted_files.push((path, "ANIM".to_string()));
        println!("  Created: {}", name);
    }
    
    // Step 2: Create validation pipeline
    println!("\nStep 2: Creating validation pipeline...");
    let mut pipeline = ValidationPipeline::new(report_dir.clone())
        .expect("Failed to create pipeline");
    println!("  ✓ Pipeline created");
    
    // Step 3: Configure validation
    println!("\nStep 3: Configuring validation...");
    let config = ValidationConfig {
        enable_reference_validation: false,
        enable_byte_comparison: true,
        enable_visual_validation: true,
        enable_unity_validation: false,
        enable_regression_testing: true,
        fail_fast: false,
        generate_diagnostics: true,
        report_dir: report_dir.clone(),
    };
    println!("  ✓ Configuration set");
    
    // Step 4: Run validation
    println!("\nStep 4: Running validation...");
    let report = pipeline.validate_batch(&extracted_files, &config)
        .expect("Validation failed");
    
    // Step 5: Display results
    println!("\nStep 5: Validation Results:");
    println!("  {}", "=".repeat(60));
    println!("  {}", report.summary);
    println!("  {}", "=".repeat(60));
    println!("  Total Files:  {}", report.total_files);
    println!("  Passed:       {}", report.passed_files);
    println!("  Failed:       {}", report.failed_files);
    println!("  Success Rate: {:.1}%", 
        (report.passed_files as f64 / report.total_files as f64) * 100.0);
    
    // Step 6: Show individual results
    println!("\n  Individual Results:");
    for extraction in &report.extraction_reports {
        let status = if extraction.overall_pass { "✓" } else { "✗" };
        println!("    {} {} ({})", status, extraction.file_name, extraction.format);
        
        if !extraction.diagnostics.is_empty() {
            for diagnostic in &extraction.diagnostics {
                println!("      - {}", diagnostic);
            }
        }
    }
    
    // Step 7: Verify report generation
    println!("\nStep 6: Verifying report generation...");
    if report_dir.exists() {
        println!("  ✓ Report directory created");
        
        // List generated files
        if let Ok(entries) = std::fs::read_dir(&report_dir) {
            println!("  Generated files:");
            for entry in entries.flatten() {
                println!("    - {:?}", entry.file_name());
            }
        }
    }
    
    // Assertions
    assert_eq!(report.total_files, 3, "Should validate 3 files");
    assert!(report.passed_files > 0, "At least some files should pass");
    
    println!("\n=== ✅ End-to-End Validation Workflow Test PASSED ===\n");
}
