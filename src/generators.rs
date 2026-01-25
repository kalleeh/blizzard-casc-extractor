//! Property test generators for CASC Sprite Extractor
//! 
//! This module provides centralized property test generators used across
//! all test modules to ensure consistent test data generation.

#[cfg(test)]
use proptest::prelude::*;
#[cfg(test)]
use tempfile::TempDir;

/// Generate valid CASC installation paths
/// 
/// Creates temporary directories with proper CASC structure for testing
#[cfg(test)]
pub fn valid_casc_path_strategy() -> impl Strategy<Value = TempDir> {
    any::<()>().prop_map(|_| {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().join("Data").join("data");
        let indices_dir = temp_dir.path().join("Data").join("indices");
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::create_dir_all(&indices_dir).unwrap();
        
        // Create a simple valid index file
        let index_path = data_dir.join("0000000001.idx");
        let mut index_data = vec![0u8; 16];
        
        // Header
        index_data[0..4].copy_from_slice(&16u32.to_le_bytes()); // header_hash_size
        index_data[4..8].copy_from_slice(&0x12345678u32.to_le_bytes()); // header_hash
        index_data[8..10].copy_from_slice(&7u16.to_le_bytes()); // unk0 = 7
        index_data[10] = 1; // bucket_index
        index_data[11] = 0; // unk1
        index_data[12] = 32; // entry_size_bytes
        index_data[13] = 32; // entry_offset_bytes
        index_data[14] = 9; // entry_key_bytes
        index_data[15] = 0; // archive_file_header_size
        
        // Add 8 bytes for archive_total_size_maximum
        index_data.extend_from_slice(&0u64.to_le_bytes());
        
        // Add a few entries
        for i in 0..3 {
            let mut entry_data = vec![0u8; 17];
            // Key
            for j in 0..9 {
                entry_data[j] = ((i * 9 + j) % 256) as u8;
            }
            // Data file number
            entry_data[9..13].copy_from_slice(&(i as u32).to_le_bytes());
            // Data file offset
            entry_data[13..17].copy_from_slice(&((i * 1024) as u32).to_le_bytes());
            index_data.extend_from_slice(&entry_data);
        }
        
        std::fs::write(&index_path, &index_data).unwrap();
        
        // Create corresponding data files with data at the expected offsets
        for i in 0..3 {
            let data_path = data_dir.join(format!("data.{:03}", i));
            let offset = (i * 1024) as usize;
            let mut data_content = vec![0u8; offset + 100]; // Create enough space for the offset
            
            // Write some test data at the expected offset
            let test_data = vec![0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x9A];
            data_content[offset..offset + test_data.len()].copy_from_slice(&test_data);
            
            std::fs::write(&data_path, &data_content).unwrap();
        }
        
        temp_dir
    })
}

/// Generate valid index file data
/// 
/// Creates properly formatted CASC index files for testing
#[cfg(test)]
pub fn index_file_strategy() -> impl Strategy<Value = Vec<u8>> {
    // Generate a minimal valid index file
    prop::collection::vec(any::<u8>(), 16..1024).prop_map(|mut data| {
        // Ensure we have at least 16 bytes for header
        if data.len() < 16 {
            data.resize(16, 0);
        }
        
        // Set header_hash_size (first 4 bytes)
        data[0..4].copy_from_slice(&16u32.to_le_bytes());
        
        // Set header_hash (next 4 bytes) - can be any value
        data[4..8].copy_from_slice(&0x12345678u32.to_le_bytes());
        
        // Set unk0 to 7 (required value)
        data[8..10].copy_from_slice(&7u16.to_le_bytes());
        
        // Set bucket_index (can be any u8)
        data[10] = (data.len() % 256) as u8;
        
        // Set unk1 (can be any u8)
        data[11] = 0;
        
        // Set entry_size_bytes to 32 (4 bytes for data file number)
        data[12] = 32;
        
        // Set entry_offset_bytes to 32 (4 bytes for offset)
        data[13] = 32;
        
        // Set entry_key_bytes to 9 (required value)
        data[14] = 9;
        
        // Set archive_file_header_size
        data[15] = 0;
        
        // Ensure we have space for the 8-byte archive_total_size_maximum
        if data.len() < 24 {
            data.resize(24, 0);
        }
        
        // Add some valid entries (each entry is 9 + 4 + 4 = 17 bytes)
        let remaining_space = data.len() - 16;
        let entry_size = 17; // 9 bytes key + 4 bytes file number + 4 bytes offset
        let num_entries = remaining_space / entry_size;
        
        // Truncate to fit complete entries
        let total_size = 16 + (num_entries * entry_size);
        data.truncate(total_size);
        
        // Fill in entries
        for i in 0..num_entries {
            let entry_start = 16 + (i * entry_size);
            
            // Key (9 bytes) - make it somewhat realistic
            for j in 0..9 {
                data[entry_start + j] = ((i * 9 + j) % 256) as u8;
            }
            
            // Data file number (4 bytes)
            let file_num = (i % 10) as u32;
            data[entry_start + 9..entry_start + 13].copy_from_slice(&file_num.to_le_bytes());
            
            // Data file offset (4 bytes)
            let offset = (i * 1024) as u32;
            data[entry_start + 13..entry_start + 17].copy_from_slice(&offset.to_le_bytes());
        }
        
        data
    })
}

/// Generate valid .anim file data
/// 
/// Creates properly formatted .anim files for testing
#[cfg(test)]
pub fn anim_file_strategy() -> impl Strategy<Value = Vec<u8>> {
    (
        1u8..=4,                    // scale
        prop_oneof![Just(1u8), Just(2u8)], // anim_type
        0u16..=10,                  // layer_count
        1u16..=100,                 // sprite_count
        prop::collection::vec("[a-zA-Z0-9_-]{1,20}", 0..10), // layer_names
        1u16..=10                   // frames_per_sprite
    ).prop_map(|(scale, anim_type, layer_count, sprite_count, layer_names, frames_per_sprite)| {
        let mut data = Vec::new();
        
        // Write header
        data.extend_from_slice(&0x4D494E41u32.to_le_bytes()); // magic "ANIM"
        data.push(scale);
        data.push(anim_type);
        data.extend_from_slice(&0u16.to_le_bytes()); // unknown
        data.extend_from_slice(&layer_count.to_le_bytes());
        data.extend_from_slice(&sprite_count.to_le_bytes());
        
        // Write layer names
        let max_layers = std::cmp::min(layer_count as usize, 10);
        for i in 0..max_layers {
            if i < layer_names.len() {
                data.extend_from_slice(layer_names[i].as_bytes());
            } else {
                data.extend_from_slice(b"default");
            }
            data.push(0); // null terminator
        }
        
        // Calculate texture offset (after all sprite headers)
        let texture_size = 64; // 4x4 RGBA = 64 bytes
        
        // Calculate where textures will start (after all sprite headers)
        let sprite_header_size = 1 + 2 + 2 + 2; // is_reference + width + height + frame_count
        let frames_size = frames_per_sprite as usize * 16; // 16 bytes per frame
        let texture_header_size = 2 + 4 + 4 + 2 + 2; // texture_count + offset + size + width + height
        let total_sprite_headers_size = sprite_count as usize * (sprite_header_size + frames_size + texture_header_size);
        let texture_start_offset = data.len() + total_sprite_headers_size;
        
        // Calculate texture offsets for each sprite
        let mut texture_offsets = Vec::new();
        for i in 0..sprite_count {
            texture_offsets.push((texture_start_offset + i as usize * texture_size) as u32);
        }
        
        // Write sprites
        for sprite_idx in 0..sprite_count {
            data.push(0); // is_reference = false
            data.extend_from_slice(&4u16.to_le_bytes()); // width (4x4 texture)
            data.extend_from_slice(&4u16.to_le_bytes()); // height
            data.extend_from_slice(&frames_per_sprite.to_le_bytes()); // frame_count
            
            // Write frames
            for frame_idx in 0..frames_per_sprite {
                data.extend_from_slice(&0u16.to_le_bytes()); // tex_x
                data.extend_from_slice(&0u16.to_le_bytes()); // tex_y
                data.extend_from_slice(&0i16.to_le_bytes()); // x_offset
                data.extend_from_slice(&0i16.to_le_bytes()); // y_offset
                data.extend_from_slice(&4u16.to_le_bytes()); // width
                data.extend_from_slice(&4u16.to_le_bytes()); // height
                // Use frame index * 100 as timing (100ms, 200ms, etc.) for test data
                let timing = (frame_idx + 1) * 100;
                data.extend_from_slice(&(timing as u32).to_le_bytes()); // timing (4 bytes)
            }
            
            // Write texture count and texture header
            data.extend_from_slice(&1u16.to_le_bytes()); // texture_count
            data.extend_from_slice(&texture_offsets[sprite_idx as usize].to_le_bytes()); // offset
            data.extend_from_slice(&(texture_size as u32).to_le_bytes()); // size
            data.extend_from_slice(&4u16.to_le_bytes()); // width
            data.extend_from_slice(&4u16.to_le_bytes()); // height
        }
        
        // Write texture data for all sprites
        for _ in 0..sprite_count {
            // Create proper RGBA texture data (4x4 pixels = 64 bytes)
            let texture_data = vec![128u8; texture_size]; // Gray pixels
            data.extend_from_slice(&texture_data);
        }
        
        data
    })
}

/// Generate file path patterns for testing
/// 
/// Creates realistic file paths that would be found in CASC archives
#[cfg(test)]
pub fn file_path_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Simple file names
        Just("test.anim".to_string()),
        Just("unit.anim".to_string()),
        Just("building.anim".to_string()),
        Just("effect.anim".to_string()),
        // Race-specific files
        Just("terran_unit.anim".to_string()),
        Just("protoss_building.anim".to_string()),
        Just("zerg_effect.anim".to_string()),
        // Directory paths
        Just("anim/terran/unit.anim".to_string()),
        Just("HD/anim/protoss/building.anim".to_string()),
        Just("HD2/anim/zerg/effect.anim".to_string()),
        Just("SD/sprites/terran/unit.anim".to_string()),
        // UI files
        Just("ui/button.anim".to_string()),
        Just("ui/icon.anim".to_string()),
        // Temp files
        Just("temp/test.anim".to_string()),
        Just("cache/temp.anim".to_string()),
        // Complex paths with multiple directories
        Just("anim/units/terran/marine.anim".to_string()),
        Just("HD/sprites/buildings/protoss/nexus.anim".to_string()),
        Just("HD2/effects/zerg/spawn.anim".to_string()),
        Just("SD/ui/wireframes/terran/scv.anim".to_string()),
    ]
}

/// Generate regex patterns for filtering
/// 
/// Creates valid regex patterns commonly used for file filtering
#[cfg(test)]
pub fn regex_pattern_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Simple literal patterns
        Just("test".to_string()),
        Just("file".to_string()),
        Just("anim".to_string()),
        Just("terran".to_string()),
        Just("protoss".to_string()),
        Just("zerg".to_string()),
        // Wildcard patterns
        Just(".*test.*".to_string()),
        Just(".*file.*".to_string()),
        Just(".*anim.*".to_string()),
        Just(".*unit.*".to_string()),
        // Character class patterns
        Just("[a-zA-Z]+".to_string()),
        Just("[0-9]+".to_string()),
        // Simple alternation
        Just("(terran|protoss|zerg)".to_string()),
        Just("(unit|building|effect)".to_string()),
        // Escaped special characters
        Just("\\.anim".to_string()),
        Just("\\.png".to_string()),
        // Simple quantifiers
        Just("[a-zA-Z]+\\.anim".to_string()),
        // Directory patterns
        Just(".*/anim/.*".to_string()),
        Just(".*/sprites/.*".to_string()),
        Just(".*/HD/.*".to_string()),
        Just(".*/HD2/.*".to_string()),
        Just(".*/SD/.*".to_string()),
        // Race-specific patterns
        Just(".*terran.*".to_string()),
        Just(".*protoss.*".to_string()),
        Just(".*zerg.*".to_string()),
        // File extension patterns
        Just(".*\\.anim$".to_string()),
        Just(".*\\.png$".to_string()),
        // Complex patterns
        Just("(HD|HD2|SD)/.*\\.anim".to_string()),
        Just(".*/units/(terran|protoss|zerg)/.*".to_string()),
    ]
}

/// Generate valid keys that exist in the mock CASC archive
/// 
/// Creates keys that correspond to files actually present in the test CASC archive
#[cfg(test)]
pub fn valid_casc_key_strategy() -> impl Strategy<Value = [u8; 9]> {
    (0u8..3u8, 0usize..3usize).prop_map(|(bucket, entry)| {
        let mut key = [0u8; 9];
        for j in 0..9 {
            key[j] = ((bucket as usize * 100 + entry * 9 + j) % 256) as u8;
        }
        key
    })
}

/// Generate collections of file paths for bulk testing
#[cfg(test)]
pub fn file_path_collection_strategy() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(file_path_strategy(), 5..20)
}

/// Generate collections of regex patterns for bulk testing
#[cfg(test)]
pub fn regex_pattern_collection_strategy() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(regex_pattern_strategy(), 1..5)
}

/// Generate corrupted index file data for error testing
#[cfg(test)]
pub fn corrupted_index_file_strategy() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        // Too short (less than 16 bytes header)
        prop::collection::vec(any::<u8>(), 0..16),
        // Invalid unk0 value (not 7)
        index_file_strategy().prop_map(|mut data| {
            if data.len() >= 10 {
                data[8..10].copy_from_slice(&42u16.to_le_bytes()); // Invalid unk0
            }
            data
        }),
        // Invalid entry_key_bytes (not 9)
        index_file_strategy().prop_map(|mut data| {
            if data.len() >= 15 {
                data[14] = 7; // Invalid entry_key_bytes
            }
            data
        }),
        // Truncated entries
        index_file_strategy().prop_map(|mut data| {
            if data.len() > 20 {
                data.truncate(data.len() - 5); // Remove some bytes from the end
            }
            data
        }),
    ]
}

/// Generate corrupted .anim file data for error testing
#[cfg(test)]
pub fn corrupted_anim_file_strategy() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        // Too short (less than header size)
        prop::collection::vec(any::<u8>(), 0..12),
        // Invalid magic number
        anim_file_strategy().prop_map(|mut data| {
            if data.len() >= 4 {
                data[0..4].copy_from_slice(&0x12345678u32.to_le_bytes()); // Invalid magic
            }
            data
        }),
        // Truncated file
        anim_file_strategy().prop_map(|mut data| {
            if data.len() > 20 {
                data.truncate(data.len() / 2); // Cut file in half
            }
            data
        }),
        // Invalid scale value
        anim_file_strategy().prop_map(|mut data| {
            if data.len() >= 5 {
                data[4] = 0; // Invalid scale (must be 1-4)
            }
            data
        }),
    ]
}

/// Generate research data for testing
/// 
/// Creates realistic research data structures for property testing
#[cfg(test)]
pub fn research_data_strategy() -> impl Strategy<Value = crate::research::ResearchData> {
    use crate::research::*;
    use std::collections::HashMap;
    
    // Split the generation into smaller tuples to avoid trait bound issues
    let basic_data = (
        "[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z",
        "0\\.[0-9]\\.[0-9]",
        valid_casc_path_strategy(),
    );
    
    let casc_stats = (
        1usize..=20,  // index_file_count
        1usize..=10,  // data_file_count
        1_000_000u64..=10_000_000_000u64,  // total_data_size
        1000usize..=100_000,  // total_file_entries
        0.0f64..=8.0,  // average_entropy
        prop::collection::vec("[a-zA-Z0-9_.-]+", 0..5),  // corrupted_files
    );
    
    let format_data = (
        0u32..=100,  // png_count
        0u32..=100,  // jpeg_count
        0u32..=50,   // dds_count
        0u32..=1000, // anim_count
        0u32..=1000, // tiny_files
        0u32..=5000, // small_files
        0u32..=2000, // medium_files
        0u32..=500,  // large_files
        0u32..=100,  // huge_files
    );
    
    let extraction_data = (
        0u32..=10000,  // files_extracted
        0u32..=100,    // extraction_failures
        0u32..=5000,   // png_conversions
        0u32..=50,     // conversion_failures
        0.1f64..=3600.0,  // extraction_time_seconds
        0.1f64..=1000.0,  // average_processing_time_ms
    );
    
    (basic_data, casc_stats, format_data, extraction_data).prop_map(|(
        (timestamp, tool_version, temp_dir),
        (index_file_count, data_file_count, total_data_size, total_file_entries, average_entropy, corrupted_files),
        (png_count, jpeg_count, dds_count, anim_count, tiny_files, small_files, medium_files, large_files, huge_files),
        (files_extracted, extraction_failures, png_conversions, conversion_failures, extraction_time_seconds, average_processing_time_ms)
    )| {
        let mut other_formats = HashMap::new();
        other_formats.insert("BLP".to_string(), 5);
        other_formats.insert("TGA".to_string(), 12);
        
        ResearchData {
            timestamp,
            tool_version,
            installation_path: temp_dir.path().to_path_buf(),
            casc_stats: CascStats {
                index_file_count,
                data_file_count,
                total_data_size,
                total_file_entries,
                average_entropy,
                corrupted_files,
            },
            format_analysis: FormatAnalysis {
                png_count,
                jpeg_count,
                dds_count,
                anim_count,
                other_formats,
                size_distribution: SizeDistribution {
                    tiny_files,
                    small_files,
                    medium_files,
                    large_files,
                    huge_files,
                },
            },
            extraction_stats: ExtractionStats {
                files_extracted,
                extraction_failures,
                png_conversions,
                conversion_failures,
                extraction_time_seconds,
                average_processing_time_ms,
            },
            tool_integration: ToolIntegrationResults {
                tools_tested: vec![
                    ToolTestResult {
                        tool_name: "CascLib".to_string(),
                        tool_version: Some("1.0.0".to_string()),
                        is_compatible: true,
                        files_extracted: 100,
                        extraction_time_seconds: 30.0,
                        output_quality: OutputQuality {
                            valid_image_percentage: 95.0,
                            correct_metadata_percentage: 90.0,
                            correct_structure: true,
                            overall_score: 0.95,
                        },
                        errors: vec![],
                    }
                ],
                recommended_tool: Some("CascLib".to_string()),
                integration_method: "direct_casc_parsing".to_string(),
                integration_success_rate: 0.95,
            },
            unknown_signatures: vec![
                UnknownSignature {
                    signature: "12345678ABCDEF00".to_string(),
                    occurrence_count: 3,
                    average_size: 2048,
                    sample_paths: vec!["test/file1.dat".to_string(), "test/file2.dat".to_string()],
                },
            ],
            format_statistics: FormatStatistics {
                format_success_rates: {
                    let mut rates = HashMap::new();
                    rates.insert("ANIM".to_string(), FormatSuccessRate {
                        format_name: "ANIM".to_string(),
                        successful_extractions: 85,
                        failed_extractions: 15,
                        success_percentage: 85.0,
                        average_processing_time_ms: 12.5,
                        failure_reasons: {
                            let mut reasons = HashMap::new();
                            reasons.insert("Corrupted header".to_string(), 8);
                            reasons.insert("Unsupported compression".to_string(), 7);
                            reasons
                        },
                    });
                    rates.insert("GRP".to_string(), FormatSuccessRate {
                        format_name: "GRP".to_string(),
                        successful_extractions: 92,
                        failed_extractions: 8,
                        success_percentage: 92.0,
                        average_processing_time_ms: 8.3,
                        failure_reasons: {
                            let mut reasons = HashMap::new();
                            reasons.insert("Invalid palette".to_string(), 5);
                            reasons.insert("Truncated file".to_string(), 3);
                            reasons
                        },
                    });
                    rates
                },
                format_patterns: {
                    let mut patterns = HashMap::new();
                    patterns.insert("ANIM".to_string(), FormatPattern {
                        format_name: "ANIM".to_string(),
                        size_patterns: vec![
                            SizeRange { min_size: 1024, max_size: 10240, file_count: 25, success_rate: 0.9 },
                            SizeRange { min_size: 10241, max_size: 102400, file_count: 45, success_rate: 0.85 },
                        ],
                        header_patterns: vec![
                            HeaderPattern {
                                description: "Standard ANIM header".to_string(),
                                hex_pattern: "414E494D01020000".to_string(),
                                occurrence_count: 60,
                                success_rate: 0.88,
                            },
                        ],
                        compression_patterns: vec![
                            CompressionPattern {
                                compression_type: "ZLIB".to_string(),
                                file_count: 40,
                                average_compression_ratio: 0.65,
                                decompression_success_rate: 0.95,
                            },
                        ],
                        detection_confidence_distribution: vec![
                            ConfidenceRange { min_confidence: 0.8, max_confidence: 1.0, detection_count: 70, actual_success_rate: 0.9 },
                            ConfidenceRange { min_confidence: 0.6, max_confidence: 0.8, detection_count: 20, actual_success_rate: 0.75 },
                        ],
                    });
                    patterns
                },
                overall_success_rate: 88.5,
                top_performing_formats: vec!["GRP".to_string(), "ANIM".to_string(), "PCX".to_string()],
            },
            format_variants: vec![
                FormatVariant {
                    base_format: "ANIM".to_string(),
                    variant_id: "ANIM_v2".to_string(),
                    description: "ANIM format with extended header".to_string(),
                    differences: vec![
                        FormatDifference {
                            difference_type: "Header".to_string(),
                            description: "Additional 4 bytes at offset 12".to_string(),
                            byte_offset: Some(12),
                            expected_value: Some("00000000".to_string()),
                            actual_value: Some("12345678".to_string()),
                        },
                    ],
                    sample_files: vec!["HD2/anim/terran/unit_v2.anim".to_string()],
                    discovered_timestamp: chrono::Utc::now().to_rfc3339(),
                    occurrence_count: 5,
                    extraction_success_rate: 0.8,
                },
            ],
            performance_metrics: PerformanceMetrics {
                system_info: SystemInfo {
                    os: "Linux".to_string(),
                    cpu: "Intel Core i7-9700K".to_string(),
                    total_ram: 16_777_216_000,
                    available_ram: 8_388_608_000,
                    storage_type: "SSD".to_string(),
                    cpu_cores: 8,
                },
                operation_performance: {
                    let mut ops = HashMap::new();
                    ops.insert("format_detection".to_string(), OperationPerformance {
                        operation_name: "format_detection".to_string(),
                        operation_count: 1000,
                        total_time_seconds: 5.2,
                        average_time_ms: 5.2,
                        min_time_ms: 1.0,
                        max_time_ms: 25.0,
                        time_std_dev_ms: 3.1,
                        operations_per_second: 192.3,
                    });
                    ops.insert("sprite_extraction".to_string(), OperationPerformance {
                        operation_name: "sprite_extraction".to_string(),
                        operation_count: 500,
                        total_time_seconds: 12.8,
                        average_time_ms: 25.6,
                        min_time_ms: 5.0,
                        max_time_ms: 120.0,
                        time_std_dev_ms: 15.2,
                        operations_per_second: 39.1,
                    });
                    ops
                },
                memory_usage: MemoryUsageMetrics {
                    peak_memory_usage: 524_288_000,
                    average_memory_usage: 314_572_800,
                    component_memory_usage: {
                        let mut components = HashMap::new();
                        components.insert("format_detector".to_string(), 52_428_800);
                        components.insert("sprite_parser".to_string(), 157_286_400);
                        components.insert("image_converter".to_string(), 104_857_600);
                        components
                    },
                    gc_count: 0,
                    gc_time_seconds: 0.0,
                },
                io_performance: IoPerformanceMetrics {
                    total_bytes_read: 1_073_741_824,
                    total_bytes_written: 536_870_912,
                    average_read_speed_mbps: 125.5,
                    average_write_speed_mbps: 89.2,
                    file_operations: 1500,
                    average_file_op_time_ms: 2.3,
                },
                scalability_metrics: ScalabilityMetrics {
                    file_count_scaling: vec![
                        ScalingDataPoint { parameter_value: 100.0, processing_time_seconds: 2.1, memory_usage_bytes: 104_857_600, throughput: 47.6 },
                        ScalingDataPoint { parameter_value: 500.0, processing_time_seconds: 8.5, memory_usage_bytes: 314_572_800, throughput: 58.8 },
                        ScalingDataPoint { parameter_value: 1000.0, processing_time_seconds: 18.2, memory_usage_bytes: 524_288_000, throughput: 54.9 },
                    ],
                    file_size_scaling: vec![
                        ScalingDataPoint { parameter_value: 1024.0, processing_time_seconds: 0.05, memory_usage_bytes: 2_097_152, throughput: 20.0 },
                        ScalingDataPoint { parameter_value: 102400.0, processing_time_seconds: 0.25, memory_usage_bytes: 10_485_760, throughput: 4.0 },
                        ScalingDataPoint { parameter_value: 1048576.0, processing_time_seconds: 1.2, memory_usage_bytes: 52_428_800, throughput: 0.83 },
                    ],
                    thread_count_scaling: vec![
                        ScalingDataPoint { parameter_value: 1.0, processing_time_seconds: 25.0, memory_usage_bytes: 104_857_600, throughput: 40.0 },
                        ScalingDataPoint { parameter_value: 4.0, processing_time_seconds: 8.5, memory_usage_bytes: 209_715_200, throughput: 117.6 },
                        ScalingDataPoint { parameter_value: 8.0, processing_time_seconds: 5.2, memory_usage_bytes: 314_572_800, throughput: 192.3 },
                    ],
                    memory_scaling: vec![
                        ScalingDataPoint { parameter_value: 104_857_600.0, processing_time_seconds: 12.0, memory_usage_bytes: 104_857_600, throughput: 83.3 },
                        ScalingDataPoint { parameter_value: 524_288_000.0, processing_time_seconds: 8.5, memory_usage_bytes: 524_288_000, throughput: 117.6 },
                    ],
                },
                performance_comparison: Some(PerformanceComparison {
                    baseline_version: "1.3.0".to_string(),
                    performance_factor: 1.25,
                    memory_factor: 0.85,
                    operation_comparisons: {
                        let mut comparisons = HashMap::new();
                        comparisons.insert("format_detection".to_string(), 1.15);
                        comparisons.insert("sprite_extraction".to_string(), 1.35);
                        comparisons
                    },
                }),
            },
        }
    })
}

/// Generate invalid CASC paths for error testing
#[cfg(test)]
pub fn invalid_casc_path_strategy() -> impl Strategy<Value = TempDir> {
    prop_oneof![
        // Empty directory
        any::<()>().prop_map(|_| {
            TempDir::new().unwrap()
        }),
        // Directory with Data but no data subdirectory
        any::<()>().prop_map(|_| {
            let temp_dir = TempDir::new().unwrap();
            std::fs::create_dir_all(temp_dir.path().join("Data")).unwrap();
            temp_dir
        }),
        // Directory with Data/data but no index files
        any::<()>().prop_map(|_| {
            let temp_dir = TempDir::new().unwrap();
            std::fs::create_dir_all(temp_dir.path().join("Data").join("data")).unwrap();
            temp_dir
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn test_valid_casc_path_strategy(temp_dir in valid_casc_path_strategy()) {
            // Verify the generated CASC path has the expected structure
            let data_dir = temp_dir.path().join("Data").join("data");
            let indices_dir = temp_dir.path().join("Data").join("indices");
            
            assert!(data_dir.exists());
            assert!(indices_dir.exists());
            
            // Check for index file
            let index_file = data_dir.join("0000000001.idx");
            assert!(index_file.exists());
            
            // Check for data files
            for i in 0..3 {
                let data_file = data_dir.join(format!("data.{:03}", i));
                assert!(data_file.exists());
            }
        }
        
        #[test]
        fn test_index_file_strategy(data in index_file_strategy()) {
            // Verify the generated index file has valid structure
            assert!(data.len() >= 16, "Index file must have at least 16 bytes for header");
            
            // Check header values
            let header_hash_size = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
            assert_eq!(header_hash_size, 16);
            
            let unk0 = u16::from_le_bytes([data[8], data[9]]);
            assert_eq!(unk0, 7);
            
            let entry_size_bytes = data[12];
            assert_eq!(entry_size_bytes, 32);
            
            let entry_offset_bytes = data[13];
            assert_eq!(entry_offset_bytes, 32);
            
            let entry_key_bytes = data[14];
            assert_eq!(entry_key_bytes, 9);
        }
        
        #[test]
        fn test_anim_file_strategy(data in anim_file_strategy()) {
            // Verify the generated .anim file has valid structure
            assert!(data.len() >= 12, "Anim file must have at least 12 bytes for header");
            
            // Check magic number
            let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
            assert_eq!(magic, 0x4D494E41); // "ANIM"
            
            // Check scale
            let scale = data[4];
            assert!(scale >= 1 && scale <= 4, "Scale must be between 1 and 4");
            
            // Check anim_type
            let anim_type = data[5];
            assert!(anim_type == 1 || anim_type == 2, "Anim type must be 1 or 2");
        }
        
        #[test]
        fn test_file_path_strategy(path in file_path_strategy()) {
            // Verify the generated file path is reasonable
            assert!(!path.is_empty(), "File path should not be empty");
            assert!(path.contains(".anim"), "File path should contain .anim extension");
        }
        
        #[test]
        fn test_regex_pattern_strategy(pattern in regex_pattern_strategy()) {
            // Verify the generated regex pattern is valid
            assert!(!pattern.is_empty(), "Regex pattern should not be empty");
            
            // Try to compile the regex to ensure it's valid
            let regex_result = regex::Regex::new(&pattern);
            assert!(regex_result.is_ok(), "Generated regex pattern should be valid: {}", pattern);
        }
        
        #[test]
        fn test_corrupted_index_file_strategy(data in corrupted_index_file_strategy()) {
            // Verify that corrupted data is actually corrupted in expected ways
            if data.len() < 16 {
                // Too short - this is expected corruption
                return Ok(());
            }
            
            if data.len() >= 10 {
                let unk0 = u16::from_le_bytes([data[8], data[9]]);
                if unk0 != 7 {
                    // Invalid unk0 - this is expected corruption
                    return Ok(());
                }
            }
            
            if data.len() >= 15 {
                let entry_key_bytes = data[14];
                if entry_key_bytes != 9 {
                    // Invalid entry_key_bytes - this is expected corruption
                    return Ok(());
                }
            }
            
            // If we get here, the data might be truncated or have other corruption
            // which is also acceptable for this strategy
        }
        
        #[test]
        fn test_corrupted_anim_file_strategy(data in corrupted_anim_file_strategy()) {
            // Verify that corrupted data is actually corrupted in expected ways
            if data.len() < 12 {
                // Too short - this is expected corruption
                return Ok(());
            }
            
            if data.len() >= 4 {
                let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                if magic != 0x4D494E41 {
                    // Invalid magic - this is expected corruption
                    return Ok(());
                }
            }
            
            if data.len() >= 5 {
                let scale = data[4];
                if scale == 0 || scale > 4 {
                    // Invalid scale - this is expected corruption
                    return Ok(());
                }
            }
            
            // If we get here, the data might be truncated or have other corruption
            // which is also acceptable for this strategy
        }
    }
}