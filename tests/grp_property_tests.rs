// Property-based tests for GRP format parsing
// Validates requirements 17.6, 2.1, 2.2, 2.3

use proptest::prelude::*;
use proptest::strategy::ValueTree;

mod property_test_generators;
use property_test_generators::valid_grp_data;

// Import GRP types
use casc_extractor::grp::GrpFile;
use casc_extractor::anim::AnimPalette;

/// Property 1: GRP parsing never panics on valid input
/// Validates requirement 17.6 (property-based testing), 2.1 (GRP parsing)
mod grp_parsing_never_panics {
    use super::*;
    
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn grp_parsing_never_panics_on_valid_input(test_data in valid_grp_data()) {
            // Property: Parsing valid GRP data should never panic
            let result = std::panic::catch_unwind(|| {
                GrpFile::parse(&test_data.data)
            });
            
            // Should not panic
            prop_assert!(result.is_ok(), "GRP parsing panicked on valid input");
            
            // If parsing succeeded, verify basic structure
            if let Ok(Ok(grp)) = result {
                prop_assert_eq!(grp.frame_count, test_data.frame_count);
                prop_assert_eq!(grp.width, test_data.width);
                prop_assert_eq!(grp.height, test_data.height);
            }
        }
    }
}

/// Property 2: GRP parsing preserves dimensions across all frames
/// Validates requirement 2.1 (GRP parsing), 2.2 (frame extraction)
mod grp_dimension_preservation {
    use super::*;
    
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn grp_dimensions_preserved_across_frames(test_data in valid_grp_data()) {
            let grp = GrpFile::parse(&test_data.data).expect("Should parse valid GRP");
            
            // Property: All frames must have the same dimensions as the header
            for (i, frame) in grp.frames.iter().enumerate() {
                prop_assert_eq!(
                    frame.width, 
                    test_data.width,
                    "Frame {} width mismatch", i
                );
                prop_assert_eq!(
                    frame.height, 
                    test_data.height,
                    "Frame {} height mismatch", i
                );
                
                // Property: Pixel data size must match dimensions
                let expected_pixels = (test_data.width as usize) * (test_data.height as usize);
                prop_assert_eq!(
                    frame.pixel_data.len(),
                    expected_pixels,
                    "Frame {} pixel count mismatch", i
                );
            }
        }
    }
}

/// Property 3: Frame extraction completeness
/// Validates requirement 2.2 (frame extraction completeness)
mod grp_frame_extraction_completeness {
    use super::*;
    
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn all_frames_extracted_successfully(test_data in valid_grp_data()) {
            let grp = GrpFile::parse(&test_data.data).expect("Should parse valid GRP");
            
            // Property: Number of extracted frames must match header frame count
            prop_assert_eq!(
                grp.frames.len(),
                test_data.frame_count as usize,
                "Frame count mismatch"
            );
            
            // Property: All frames must be accessible
            for i in 0..test_data.frame_count as usize {
                prop_assert!(
                    grp.get_frame(i).is_some(),
                    "Frame {} should be accessible", i
                );
            }
            
            // Property: First frame must always be accessible
            prop_assert!(
                grp.get_first_frame().is_some(),
                "First frame should always be accessible"
            );
        }
    }
}

/// Property 4: Palette conversion correctness
/// Validates requirement 2.3 (palette conversion)
mod grp_palette_conversion {
    use super::*;
    
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn palette_conversion_produces_valid_rgba(test_data in valid_grp_data()) {
            let grp = GrpFile::parse(&test_data.data).expect("Should parse valid GRP");
            let palette = AnimPalette::default_starcraft_unit_palette();
            
            // Property: Each frame converts to valid RGBA data
            for (i, frame) in grp.frames.iter().enumerate() {
                let rgba_data = frame.to_rgba_with_palette(&palette)
                    .expect(&format!("Frame {} should convert to RGBA", i));
                
                // Property: RGBA data size must be 4x pixel count
                let expected_size = frame.pixel_data.len() * 4;
                prop_assert_eq!(
                    rgba_data.len(),
                    expected_size,
                    "Frame {} RGBA size mismatch", i
                );
                
                // Property: RGBA data must be divisible by 4
                prop_assert_eq!(
                    rgba_data.len() % 4,
                    0,
                    "Frame {} RGBA data not aligned to 4 bytes", i
                );
            }
        }
    }
}

/// Property 5: Transparency preservation
/// Validates requirement 2.3 (palette conversion with transparency)
mod grp_transparency_preservation {
    use super::*;
    
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn transparency_preserved_in_conversion(test_data in valid_grp_data()) {
            let grp = GrpFile::parse(&test_data.data).expect("Should parse valid GRP");
            let palette = AnimPalette::default_starcraft_unit_palette();
            
            // Property: Index 0 must always be transparent
            for (i, frame) in grp.frames.iter().enumerate() {
                let rgba_data = frame.to_rgba_with_transparency(&palette)
                    .expect(&format!("Frame {} should convert with transparency", i));
                
                // Check each pixel
                for (pixel_idx, &palette_index) in frame.pixel_data.iter().enumerate() {
                    let rgba_offset = pixel_idx * 4;
                    let alpha = rgba_data[rgba_offset + 3];
                    
                    if palette_index == 0 {
                        // Property: Index 0 must have alpha = 0
                        prop_assert_eq!(
                            alpha, 0,
                            "Frame {}, pixel {} (index 0) should be transparent", i, pixel_idx
                        );
                    } else {
                        // Property: Non-zero indices must have alpha = 255
                        prop_assert_eq!(
                            alpha, 255,
                            "Frame {}, pixel {} (index {}) should be opaque", i, pixel_idx, palette_index
                        );
                    }
                }
            }
        }
    }
}

/// Property 6: RLE decoding consistency
/// Validates requirement 2.1 (RLE decoding), 2.2 (frame extraction)
mod grp_rle_decoding_consistency {
    use super::*;
    
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn rle_decoding_produces_consistent_results(test_data in valid_grp_data()) {
            let grp = GrpFile::parse(&test_data.data).expect("Should parse valid GRP");
            
            // Property: Parsing the same data multiple times produces identical results
            let grp2 = GrpFile::parse(&test_data.data).expect("Should parse valid GRP again");
            
            prop_assert_eq!(grp.frame_count, grp2.frame_count);
            prop_assert_eq!(grp.width, grp2.width);
            prop_assert_eq!(grp.height, grp2.height);
            prop_assert_eq!(grp.frames.len(), grp2.frames.len());
            
            // Property: Each frame's pixel data must be identical
            for i in 0..grp.frames.len() {
                prop_assert_eq!(
                    &grp.frames[i].pixel_data,
                    &grp2.frames[i].pixel_data,
                    "Frame {} pixel data mismatch on re-parse", i
                );
            }
        }
    }
}

/// Property 7: Multi-frame conversion consistency
/// Validates requirement 2.2 (frame extraction), 2.3 (palette conversion)
mod grp_multi_frame_conversion {
    use super::*;
    
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn multi_frame_conversion_matches_individual_conversion(test_data in valid_grp_data()) {
            let grp = GrpFile::parse(&test_data.data).expect("Should parse valid GRP");
            let palette = AnimPalette::default_starcraft_unit_palette();
            
            // Convert all frames at once
            let all_frames_rgba = grp.convert_all_frames_to_rgba(&palette)
                .expect("Should convert all frames");
            
            // Property: Batch conversion must match individual conversions
            prop_assert_eq!(
                all_frames_rgba.len(),
                grp.frames.len(),
                "Batch conversion frame count mismatch"
            );
            
            for (i, frame) in grp.frames.iter().enumerate() {
                let individual_rgba = frame.to_rgba_with_palette(&palette)
                    .expect(&format!("Frame {} should convert individually", i));
                
                prop_assert_eq!(
                    &all_frames_rgba[i],
                    &individual_rgba,
                    "Frame {} batch vs individual conversion mismatch", i
                );
            }
        }
    }
}

/// Property 8: Optimized conversion equivalence
/// Validates requirement 2.3 (palette conversion optimization)
mod grp_optimized_conversion_equivalence {
    use super::*;
    
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn optimized_conversion_matches_regular_conversion(test_data in valid_grp_data()) {
            let grp = GrpFile::parse(&test_data.data).expect("Should parse valid GRP");
            let palette = AnimPalette::default_starcraft_unit_palette();
            
            // Property: Optimized and regular conversion must produce identical results
            for (i, frame) in grp.frames.iter().enumerate() {
                let regular_rgba = frame.to_rgba_with_transparency(&palette)
                    .expect(&format!("Frame {} should convert regularly", i));
                
                let optimized_rgba = frame.to_rgba_optimized(&palette)
                    .expect(&format!("Frame {} should convert optimized", i));
                
                prop_assert_eq!(
                    regular_rgba,
                    optimized_rgba,
                    "Frame {} optimized vs regular conversion mismatch", i
                );
            }
        }
    }
}

/// Property 9: GRP header validation
/// Validates requirement 2.1 (GRP header parsing)
mod grp_header_validation {
    use super::*;
    
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn grp_header_fields_are_valid(test_data in valid_grp_data()) {
            let grp = GrpFile::parse(&test_data.data).expect("Should parse valid GRP");
            
            // Property: Frame count must be positive
            prop_assert!(grp.frame_count > 0, "Frame count must be positive");
            
            // Property: Dimensions must be positive
            prop_assert!(grp.width > 0, "Width must be positive");
            prop_assert!(grp.height > 0, "Height must be positive");
            
            // Property: Dimensions must be reasonable (not exceed limits)
            prop_assert!(grp.width <= 2048, "Width must not exceed 2048");
            prop_assert!(grp.height <= 2048, "Height must not exceed 2048");
            prop_assert!(grp.frame_count <= 1000, "Frame count must not exceed 1000");
        }
    }
}

// Unit test to verify property test generators are available
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_property_test_generators_are_available() {
        // Verify we can generate test data
        let mut runner = proptest::test_runner::TestRunner::default();
        let test_data = valid_grp_data().new_tree(&mut runner).unwrap().current();
        
        // Basic validation
        assert!(test_data.frame_count > 0);
        assert!(test_data.width > 0);
        assert!(test_data.height > 0);
        assert!(!test_data.data.is_empty());
    }
}
