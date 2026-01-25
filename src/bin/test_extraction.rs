use std::path::Path;
use anyhow::Result;
use casc_extractor::{UnifiedPipeline, ExtractionConfig};

fn main() -> Result<()> {
    env_logger::init();
    
    println!("🚀 CASC Sprite Extraction Test");
    println!("Testing extraction of 100 sprites to demonstrate system capabilities");
    
    // Check if we have a StarCraft installation
    let potential_paths = [
        "/Applications/StarCraft/",
        "/Applications/StarCraft Remastered/",
        "C:\\Program Files (x86)\\StarCraft\\",
        "C:\\Program Files\\StarCraft\\",
        "/opt/starcraft/",
        "/usr/local/games/starcraft/",
    ];
    
    let mut casc_path = None;
    for path in &potential_paths {
        let p = Path::new(path);
        if p.exists() {
            println!("✅ Found potential StarCraft installation at: {}", path);
            casc_path = Some(p);
            break;
        }
    }
    
    let casc_path = match casc_path {
        Some(path) => path,
        None => {
            println!("❌ No StarCraft installation found in standard locations");
            println!("📝 Creating mock test data for demonstration...");
            
            // Create a mock CASC structure for testing
            let test_dir = Path::new("extracted/test_casc");
            create_mock_casc_structure(test_dir)?;
            test_dir
        }
    };
    
    // Set up extraction configuration
    let mut config = ExtractionConfig::default();
    
    // Limit to 100 files for testing
    config.filter_settings.max_files = Some(100);
    
    // Enable Unity export
    config.output_settings.unity_settings.enabled = true;
    config.output_settings.unity_settings.pixels_per_unit = 100.0;
    
    // Enable research data collection
    config.research_settings.collect_research_data = true;
    
    // Set up output directory
    let output_dir = Path::new("extracted/sprite_test_output");
    std::fs::create_dir_all(output_dir)?;
    
    println!("📁 Output directory: {:?}", output_dir);
    println!("⚙️  Configuration: Unity export enabled, max 100 files");
    
    // Initialize and run the unified pipeline
    println!("🔧 Initializing unified extraction pipeline...");
    let mut pipeline = UnifiedPipeline::new(casc_path, config)?;
    
    println!("✅ Pipeline validation...");
    pipeline.validate_configuration()?;
    
    println!("🚀 Starting extraction process...");
    let start_time = std::time::Instant::now();
    
    let result = pipeline.execute(output_dir)?;
    
    let duration = start_time.elapsed();
    
    // Print results
    println!("\n🎉 Extraction Complete!");
    println!("⏱️  Duration: {:.2} seconds", duration.as_secs_f64());
    println!("📊 Results Summary:");
    println!("   • Files processed: {}", result.metrics.files_processed);
    println!("   • Successful extractions: {}", result.processed_files.len());
    println!("   • Failed extractions: {}", result.failed_files.len());
    println!("   • Success rate: {:.1}%", 
        (result.processed_files.len() as f64 / result.metrics.files_processed as f64) * 100.0);
    
    if !result.processed_files.is_empty() {
        println!("\n📁 Successfully extracted files:");
        for (i, file) in result.processed_files.iter().take(10).enumerate() {
            println!("   {}. {} -> {:?}", i + 1, file.source_path, file.output_path.file_name().unwrap_or_default());
        }
        if result.processed_files.len() > 10 {
            println!("   ... and {} more files", result.processed_files.len() - 10);
        }
    }
    
    if !result.failed_files.is_empty() {
        println!("\n⚠️  Failed extractions (first 5):");
        for (i, file) in result.failed_files.iter().take(5).enumerate() {
            println!("   {}. {} - {}", i + 1, file.source_path, file.error_message);
        }
        if result.failed_files.len() > 5 {
            println!("   ... and {} more failures", result.failed_files.len() - 5);
        }
    }
    
    // Performance metrics
    println!("\n📈 Performance Metrics:");
    println!("   • Average processing time: {:.2}ms per file", result.metrics.average_processing_time_ms);
    println!("   • Total processing time: {:.2}s", result.metrics.total_processing_time);
    
    // Format detection stats
    if !result.metrics.format_detection_stats.is_empty() {
        println!("\n🔍 Format Detection Statistics:");
        for (format, count) in &result.metrics.format_detection_stats {
            println!("   • {}: {} files", format, count);
        }
    }
    
    println!("\n✅ Test completed successfully!");
    println!("📁 Check the 'extracted/sprite_test_output' directory for results");
    
    Ok(())
}

fn create_mock_casc_structure(casc_path: &Path) -> Result<()> {
    println!("🏗️  Creating mock CASC structure for testing...");
    
    let data_dir = casc_path.join("Data").join("data");
    std::fs::create_dir_all(&data_dir)?;
    
    // Create mock CASC index file
    let index_path = data_dir.join("data.000.idx");
    let mut index_data = vec![0u8; 1024]; // Larger mock index
    
    // Add some mock entries
    for i in 0..10 {
        let offset = i * 24;
        if offset + 24 <= index_data.len() {
            // Mock hash
            index_data[offset..offset + 8].copy_from_slice(&(i as u64).to_le_bytes());
            // Mock size
            index_data[offset + 8..offset + 10].copy_from_slice(&(100u16 + i as u16).to_le_bytes());
            // Mock flags
            index_data[offset + 14] = 9;
        }
    }
    
    std::fs::write(&index_path, &index_data)?;
    
    // Create mock CASC data file with some sprite-like data
    let data_path = data_dir.join("data.000");
    let mut mock_data = Vec::new();
    
    // Add some mock sprite data patterns
    for i in 0..100 {
        // Mock PNG header
        mock_data.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
        // Mock data
        mock_data.extend_from_slice(&format!("mock_sprite_data_{}", i).as_bytes());
        // Padding
        mock_data.resize(mock_data.len() + 100, 0);
    }
    
    std::fs::write(&data_path, &mock_data)?;
    
    println!("✅ Mock CASC structure created at: {:?}", casc_path);
    
    Ok(())
}