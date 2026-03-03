use casc_extractor::grp::{GrpFile, GrpFrame};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use png::Compression;
use regex::Regex;
use std::io::BufWriter;

// ---------------------------------------------------------------------------
// Synthetic GRP data builders
// ---------------------------------------------------------------------------

/// Build a minimal valid GRP byte buffer.
///
/// Layout:
///   6-byte header: frame_count(2) | width(2) | height(2)
///   frame_count × 8-byte entries: x(1) y(1) unk(2) fileOffset(4)
///   frame data for each frame:
///     line-offset table: height × 2 bytes
///     RLE lines: each line is `ceil(width / 63)` runs of `[0x40+count, value]`
fn build_grp(frame_count: u16, width: u16, height: u16) -> Vec<u8> {
    let h = height as usize;
    let w = width as usize;

    // Each RLE line: full-width runs of up to 63 pixels each.
    // A run is encoded as [0x40 + count, pixel_value] — 2 bytes.
    let full_runs = w / 63;
    let remainder = w % 63;
    let bytes_per_line = full_runs * 2 + if remainder > 0 { 2 } else { 0 };

    let line_table_bytes = h * 2;
    let frame_body_bytes = line_table_bytes + h * bytes_per_line;

    // Compute where each frame starts in the file.
    let header_bytes = 6;
    let entry_table_bytes = frame_count as usize * 8;
    let first_frame_offset = header_bytes + entry_table_bytes;

    let mut data: Vec<u8> = Vec::new();

    // Header
    data.extend_from_slice(&frame_count.to_le_bytes());
    data.extend_from_slice(&width.to_le_bytes());
    data.extend_from_slice(&height.to_le_bytes());

    // Frame entry table
    for i in 0..frame_count as usize {
        let offset = (first_frame_offset + i * frame_body_bytes) as u32;
        data.push(0); // x offset
        data.push(0); // y offset
        data.extend_from_slice(&0u16.to_le_bytes()); // unknown
        data.extend_from_slice(&offset.to_le_bytes());
    }

    // Frame bodies
    for frame_idx in 0..frame_count as usize {
        // Line offset table
        for line in 0..h {
            let line_offset = (line_table_bytes + line * bytes_per_line) as u16;
            data.extend_from_slice(&line_offset.to_le_bytes());
        }
        // RLE lines
        for line in 0..h {
            let pixel_value = ((frame_idx + line) % 200 + 1) as u8;
            // Full runs of 63
            for _ in 0..full_runs {
                data.push(0x40 + 63); // 0x7f
                data.push(pixel_value);
            }
            // Remaining pixels
            if remainder > 0 {
                data.push(0x40 + remainder as u8);
                data.push(pixel_value);
            }
        }
    }

    data
}

/// Build a `GrpFrame` directly without parsing, for benchmarking `to_rgba` in isolation.
fn make_frame(width: u16, height: u16) -> GrpFrame {
    let pixel_count = width as usize * height as usize;
    // Vary pixel values so the palette lookup exercises different paths.
    let pixel_data: Vec<u8> = (0..pixel_count)
        .map(|i| ((i * 7 + 13) % 256) as u8)
        .collect();
    GrpFrame {
        pixel_data,
        width,
        height,
    }
}

// ---------------------------------------------------------------------------
// Filter helpers — inlined from main.rs (private fns, not exported)
// ---------------------------------------------------------------------------

fn compile_patterns(patterns: &[&str]) -> Vec<Regex> {
    patterns
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .collect()
}

fn passes_filter(path: &str, include: &[Regex], exclude: &[Regex]) -> bool {
    if !include.is_empty() && !include.iter().any(|r| r.is_match(path)) {
        return false;
    }
    !exclude.iter().any(|r| r.is_match(path))
}

// ---------------------------------------------------------------------------
// Benchmark 1 — GRP parsing
// ---------------------------------------------------------------------------

fn bench_grp_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("grp_parse");

    // 32×32, 1 frame — small sprite
    let data_small = build_grp(1, 32, 32);
    group.throughput(Throughput::Bytes(data_small.len() as u64));
    group.bench_function(BenchmarkId::new("32x32_1frame", ""), |b| {
        b.iter(|| GrpFile::parse(std::hint::black_box(&data_small)).unwrap())
    });

    // 128×128, 10 frames — typical unit sprite
    let data_large = build_grp(10, 128, 128);
    group.throughput(Throughput::Bytes(data_large.len() as u64));
    group.bench_function(BenchmarkId::new("128x128_10frames", ""), |b| {
        b.iter(|| GrpFile::parse(std::hint::black_box(&data_large)).unwrap())
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark 2 — GrpFrame::to_rgba
// ---------------------------------------------------------------------------

fn bench_grp_to_rgba(c: &mut Criterion) {
    let mut group = c.benchmark_group("grp_to_rgba");

    let frame_small = make_frame(32, 32);
    group.throughput(Throughput::Elements(
        (frame_small.width as u64) * (frame_small.height as u64),
    ));
    group.bench_function(BenchmarkId::new("32x32", ""), |b| {
        b.iter(|| std::hint::black_box(&frame_small).to_rgba().unwrap())
    });

    let frame_medium = make_frame(128, 128);
    group.throughput(Throughput::Elements(
        (frame_medium.width as u64) * (frame_medium.height as u64),
    ));
    group.bench_function(BenchmarkId::new("128x128", ""), |b| {
        b.iter(|| std::hint::black_box(&frame_medium).to_rgba().unwrap())
    });

    let frame_large = make_frame(256, 256);
    group.throughput(Throughput::Elements(
        (frame_large.width as u64) * (frame_large.height as u64),
    ));
    group.bench_function(BenchmarkId::new("256x256", ""), |b| {
        b.iter(|| std::hint::black_box(&frame_large).to_rgba().unwrap())
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark 3 — Filter pattern matching against 1000 CASC paths
// ---------------------------------------------------------------------------

/// Generate a list of 1000 synthetic CASC-style paths.
fn make_casc_paths() -> Vec<String> {
    let prefixes = [
        "SD/unit/terran/marine",
        "HD2/unit/zerg/zergling",
        "unit/protoss/zealot",
        "tileset/badlands",
        "effect/explosion",
        "sound/terran/marine",
        "sound/zerg/zergling",
        "tileset/space/platform",
        "unit/neutral/critter",
        "glue/button",
    ];
    (0..1000)
        .map(|i| format!("{}/{:04}.grp", prefixes[i % prefixes.len()], i))
        .collect()
}

fn bench_filter(c: &mut Criterion) {
    let paths = make_casc_paths();

    let mut group = c.benchmark_group("filter_passes");
    group.throughput(Throughput::Elements(paths.len() as u64));

    // Case A: no patterns — everything passes
    group.bench_function("0_patterns", |b| {
        let include: Vec<Regex> = vec![];
        let exclude: Vec<Regex> = vec![];
        b.iter(|| {
            let mut count = 0usize;
            for path in &paths {
                if passes_filter(std::hint::black_box(path), &include, &exclude) {
                    count += 1;
                }
            }
            count
        });
    });

    // Case B: 1 include pattern
    group.bench_function("1_include_pattern", |b| {
        let include = compile_patterns(&["terran"]);
        let exclude: Vec<Regex> = vec![];
        b.iter(|| {
            let mut count = 0usize;
            for path in &paths {
                if passes_filter(std::hint::black_box(path), &include, &exclude) {
                    count += 1;
                }
            }
            count
        });
    });

    // Case C: 5 patterns (3 include, 2 exclude)
    group.bench_function("5_patterns", |b| {
        let include = compile_patterns(&["terran", "zerg", "protoss"]);
        let exclude = compile_patterns(&["sound", "glue"]);
        b.iter(|| {
            let mut count = 0usize;
            for path in &paths {
                if passes_filter(std::hint::black_box(path), &include, &exclude) {
                    count += 1;
                }
            }
            count
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark 4 — PNG compression at levels 1, 6, and 9
// ---------------------------------------------------------------------------

fn bench_png_compression(c: &mut Criterion) {
    let width: u32 = 1024;
    let height: u32 = 888;
    let pixel_count = (width * height) as usize;

    // Pre-generate RGBA pixel data (not measured).
    let rgba: Vec<u8> = (0..pixel_count * 4)
        .map(|i| ((i * 3 + 7) % 256) as u8)
        .collect();

    let levels: &[(u32, &str, Compression)] = &[
        (1, "level_1_fast", Compression::Fast),
        (6, "level_6_default", Compression::Default),
        (9, "level_9_best", Compression::Best),
    ];

    let mut group = c.benchmark_group("png_write");
    group.throughput(Throughput::Bytes((pixel_count * 4) as u64));
    // PNG encoding can take well over the default 5-second limit for Best;
    // allow up to 30 seconds so Criterion can gather enough samples.
    group.measurement_time(std::time::Duration::from_secs(30));

    for &(_level_num, name, compression) in levels {
        group.bench_function(name, |b| {
            b.iter(|| {
                let mut buf: Vec<u8> = Vec::with_capacity(pixel_count * 2);
                {
                    let w = BufWriter::new(&mut buf);
                    let mut encoder = png::Encoder::new(w, width, height);
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    encoder.set_compression(std::hint::black_box(compression));
                    let mut writer = encoder.write_header().unwrap();
                    writer.write_image_data(&rgba).unwrap();
                }
                buf.len()
            })
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

criterion_group!(
    benches,
    bench_grp_parse,
    bench_grp_to_rgba,
    bench_filter,
    bench_png_compression,
);
criterion_main!(benches);
