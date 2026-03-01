// Visual validation system with pixel-perfect diff and perceptual hashing
//
// This module provides visual comparison to ensure extracted PNGs are
// visually identical to reference tool outputs.

use super::ValidationError;
use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use std::path::Path;

/// Write a red/gray pixel diff image to `output_path`.
///
/// Pixels that match between `expected` and `actual` are rendered as grayscale;
/// pixels that differ are rendered as solid red.  The two images must have the
/// same dimensions.
pub fn create_pixel_diff_image(
    expected: &DynamicImage,
    actual: &DynamicImage,
    output_path: &Path,
) -> Result<String, ValidationError> {
    let (width, height) = expected.dimensions();
    let mut diff_img = RgbaImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let pixel1 = expected.get_pixel(x, y);
            let pixel2 = actual.get_pixel(x, y);

            let diff_pixel = if pixel1 == pixel2 {
                // Matching pixel - show in grayscale
                let gray = (pixel1[0] as u16 + pixel1[1] as u16 + pixel1[2] as u16) / 3;
                Rgba([gray as u8, gray as u8, gray as u8, 255])
            } else {
                // Different pixel - highlight in red
                Rgba([255, 0, 0, 255])
            };

            diff_img.put_pixel(x, y, diff_pixel);
        }
    }

    diff_img.save(output_path)?;
    Ok(output_path.to_string_lossy().to_string())
}

/// Result of a visual comparison
#[derive(Debug, Clone)]
pub struct VisualComparisonResult {
    /// Whether the images match pixel-for-pixel
    pub pixel_perfect_match: bool,
    
    /// Perceptual hash of the first image
    pub perceptual_hash1: u64,
    
    /// Perceptual hash of the second image
    pub perceptual_hash2: u64,
    
    /// Hamming distance between perceptual hashes (0 = identical)
    pub perceptual_distance: u32,
    
    /// Number of pixels that differ
    pub different_pixels: usize,
    
    /// Total number of pixels
    pub total_pixels: usize,
    
    /// Percentage of pixels that differ
    pub difference_percentage: f64,
    
    /// Path to diff image (if generated)
    pub diff_image_path: Option<String>,
    
    /// Path to side-by-side comparison (if generated)
    pub comparison_image_path: Option<String>,
    
    /// Detailed diagnostic message
    pub diagnostic: String,
}

impl VisualComparisonResult {
    /// Create a successful comparison result
    pub fn success(perceptual_hash: u64, total_pixels: usize) -> Self {
        Self {
            pixel_perfect_match: true,
            perceptual_hash1: perceptual_hash,
            perceptual_hash2: perceptual_hash,
            perceptual_distance: 0,
            different_pixels: 0,
            total_pixels,
            difference_percentage: 0.0,
            diff_image_path: None,
            comparison_image_path: None,
            diagnostic: "Images match pixel-for-pixel".to_string(),
        }
    }

    /// Create a failed comparison result
    pub fn failure(
        hash1: u64,
        hash2: u64,
        different_pixels: usize,
        total_pixels: usize,
        diagnostic: String,
    ) -> Self {
        let perceptual_distance = (hash1 ^ hash2).count_ones();
        let difference_percentage = (different_pixels as f64 / total_pixels as f64) * 100.0;

        Self {
            pixel_perfect_match: false,
            perceptual_hash1: hash1,
            perceptual_hash2: hash2,
            perceptual_distance,
            different_pixels,
            total_pixels,
            difference_percentage,
            diff_image_path: None,
            comparison_image_path: None,
            diagnostic,
        }
    }
}

/// Visual comparison utilities
pub struct VisualComparison;

impl VisualComparison {
    /// Compare two images with detailed visual analysis
    ///
    /// This performs:
    /// - Pixel-perfect comparison
    /// - Perceptual hashing for visual similarity
    /// - Diff image generation
    /// - Side-by-side comparison generation
    ///
    /// # Arguments
    /// * `image1` - Path to the first image
    /// * `image2` - Path to the second image
    /// * `generate_diff` - Whether to generate diff images
    ///
    /// # Returns
    /// Detailed visual comparison result
    pub fn compare_images(
        image1: &Path,
        image2: &Path,
        generate_diff: bool,
    ) -> Result<VisualComparisonResult, ValidationError> {
        // Load images
        let img1 = image::open(image1)?;
        let img2 = image::open(image2)?;

        // Check dimensions
        if img1.dimensions() != img2.dimensions() {
            let diagnostic = format!(
                "Image dimensions differ: {}x{} vs {}x{}",
                img1.width(),
                img1.height(),
                img2.width(),
                img2.height()
            );
            return Ok(VisualComparisonResult::failure(0, 0, 0, 0, diagnostic));
        }

        // Calculate perceptual hashes
        let hash1 = Self::calculate_perceptual_hash(&img1);
        let hash2 = Self::calculate_perceptual_hash(&img2);

        // Pixel-by-pixel comparison
        let (different_pixels, total_pixels) = Self::count_different_pixels(&img1, &img2);

        // If images match perfectly
        if different_pixels == 0 {
            return Ok(VisualComparisonResult::success(hash1, total_pixels));
        }

        // Images differ - create diagnostic
        let diagnostic = format!(
            "{} of {} pixels differ ({:.2}%)",
            different_pixels,
            total_pixels,
            (different_pixels as f64 / total_pixels as f64) * 100.0
        );

        let mut result = VisualComparisonResult::failure(
            hash1,
            hash2,
            different_pixels,
            total_pixels,
            diagnostic,
        );

        // Generate diff images if requested
        if generate_diff {
            if let Ok(diff_path) = Self::generate_diff_image(&img1, &img2) {
                result.diff_image_path = Some(diff_path);
            }

            if let Ok(comparison_path) = Self::generate_side_by_side(&img1, &img2) {
                result.comparison_image_path = Some(comparison_path);
            }
        }

        Ok(result)
    }

    /// Calculate perceptual hash (pHash) of an image
    ///
    /// Uses a simplified difference hash (dHash) algorithm:
    /// 1. Resize to 9x8 grayscale
    /// 2. Compare adjacent pixels horizontally
    /// 3. Generate 64-bit hash
    fn calculate_perceptual_hash(image: &DynamicImage) -> u64 {
        // Resize to 9x8 for dHash
        let resized = image.resize_exact(9, 8, image::imageops::FilterType::Lanczos3);
        let gray = resized.to_luma8();

        let mut hash = 0u64;
        let mut bit = 0;

        // Compare adjacent pixels horizontally
        for y in 0..8 {
            for x in 0..8 {
                let left = gray.get_pixel(x, y)[0];
                let right = gray.get_pixel(x + 1, y)[0];

                if left < right {
                    hash |= 1 << bit;
                }
                bit += 1;
            }
        }

        hash
    }

    /// Count the number of pixels that differ between two images
    fn count_different_pixels(img1: &DynamicImage, img2: &DynamicImage) -> (usize, usize) {
        let (width, height) = img1.dimensions();
        let total_pixels = (width * height) as usize;
        let mut different_pixels = 0;

        for y in 0..height {
            for x in 0..width {
                let pixel1 = img1.get_pixel(x, y);
                let pixel2 = img2.get_pixel(x, y);

                if pixel1 != pixel2 {
                    different_pixels += 1;
                }
            }
        }

        (different_pixels, total_pixels)
    }

    /// Generate a diff image highlighting differences
    ///
    /// Creates an image where:
    /// - Matching pixels are shown in grayscale
    /// - Different pixels are highlighted in red
    fn generate_diff_image(img1: &DynamicImage, img2: &DynamicImage) -> Result<String, ValidationError> {
        // Save diff image
        let output_dir = std::env::temp_dir().join("casc_validation");
        std::fs::create_dir_all(&output_dir)?;

        let diff_path = output_dir.join(format!(
            "diff_{}.png",
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        ));

        create_pixel_diff_image(img1, img2, &diff_path)?;
        Ok(diff_path.to_string_lossy().to_string())
    }

    /// Generate a side-by-side comparison image
    fn generate_side_by_side(img1: &DynamicImage, img2: &DynamicImage) -> Result<String, ValidationError> {
        let (width, height) = img1.dimensions();
        let mut comparison = RgbaImage::new(width * 2 + 10, height);

        // Fill with white background
        for pixel in comparison.pixels_mut() {
            *pixel = Rgba([255, 255, 255, 255]);
        }

        // Copy first image
        for y in 0..height {
            for x in 0..width {
                let pixel = img1.get_pixel(x, y);
                comparison.put_pixel(x, y, pixel);
            }
        }

        // Copy second image (with 10px gap)
        for y in 0..height {
            for x in 0..width {
                let pixel = img2.get_pixel(x, y);
                comparison.put_pixel(x + width + 10, y, pixel);
            }
        }

        // Save comparison image
        let output_dir = std::env::temp_dir().join("casc_validation");
        std::fs::create_dir_all(&output_dir)?;

        let comparison_path = output_dir.join(format!(
            "comparison_{}.png",
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        ));

        comparison.save(&comparison_path)?;
        Ok(comparison_path.to_string_lossy().to_string())
    }

    /// Calculate Hamming distance between two perceptual hashes
    pub fn hamming_distance(hash1: u64, hash2: u64) -> u32 {
        (hash1 ^ hash2).count_ones()
    }

    /// Generate a detailed visual comparison report
    pub fn generate_report(
        image1: &Path,
        image2: &Path,
        result: &VisualComparisonResult,
    ) -> String {
        let mut report = String::new();

        report.push_str("=== Visual Comparison Report ===\n\n");
        report.push_str(&format!("Image 1: {:?}\n", image1));
        report.push_str(&format!("Image 2: {:?}\n\n", image2));

        report.push_str(&format!("Perceptual Hash 1: 0x{:016X}\n", result.perceptual_hash1));
        report.push_str(&format!("Perceptual Hash 2: 0x{:016X}\n", result.perceptual_hash2));
        report.push_str(&format!("Perceptual Distance: {} bits\n\n", result.perceptual_distance));

        report.push_str(&format!("Total Pixels: {}\n", result.total_pixels));
        report.push_str(&format!("Different Pixels: {}\n", result.different_pixels));
        report.push_str(&format!("Difference: {:.4}%\n\n", result.difference_percentage));

        if result.pixel_perfect_match {
            report.push_str("✅ IMAGES MATCH PIXEL-FOR-PIXEL\n");
        } else {
            report.push_str("❌ IMAGES DIFFER\n\n");
            report.push_str(&format!("Diagnostic: {}\n", result.diagnostic));

            if let Some(diff_path) = &result.diff_image_path {
                report.push_str(&format!("\nDiff image generated: {}\n", diff_path));
            }

            if let Some(comparison_path) = &result.comparison_image_path {
                report.push_str(&format!("Side-by-side comparison: {}\n", comparison_path));
            }
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbaImage;
    use tempfile::TempDir;

    #[test]
    fn test_identical_images() {
        let temp_dir = TempDir::new().unwrap();
        let img1_path = temp_dir.path().join("img1.png");
        let img2_path = temp_dir.path().join("img2.png");

        // Create identical images
        let img = RgbaImage::from_pixel(100, 100, Rgba([128, 128, 128, 255]));
        img.save(&img1_path).unwrap();
        img.save(&img2_path).unwrap();

        let result = VisualComparison::compare_images(&img1_path, &img2_path, false).unwrap();
        assert!(result.pixel_perfect_match);
        assert_eq!(result.different_pixels, 0);
        assert_eq!(result.perceptual_hash1, result.perceptual_hash2);
    }

    #[test]
    fn test_different_images() {
        let temp_dir = TempDir::new().unwrap();
        let img1_path = temp_dir.path().join("img1.png");
        let img2_path = temp_dir.path().join("img2.png");

        // Create different images
        let img1 = RgbaImage::from_pixel(100, 100, Rgba([128, 128, 128, 255]));
        let img2 = RgbaImage::from_pixel(100, 100, Rgba([255, 0, 0, 255]));
        img1.save(&img1_path).unwrap();
        img2.save(&img2_path).unwrap();

        let result = VisualComparison::compare_images(&img1_path, &img2_path, false).unwrap();
        assert!(!result.pixel_perfect_match);
        assert_eq!(result.different_pixels, 10000); // All pixels differ
    }

    #[test]
    fn test_perceptual_hash() {
        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(100, 100, Rgba([128, 128, 128, 255])));
        let hash = VisualComparison::calculate_perceptual_hash(&img);
        assert!(hash > 0); // Hash should be non-zero for non-uniform image
    }

    #[test]
    fn test_hamming_distance() {
        let hash1 = 0b1010101010101010u64;
        let hash2 = 0b1010101010101011u64;
        let distance = VisualComparison::hamming_distance(hash1, hash2);
        assert_eq!(distance, 1); // Only 1 bit differs
    }
}
