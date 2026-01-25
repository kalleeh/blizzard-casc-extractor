# StarCraft Sprite Extraction - Technical Reference

## Overview

Complete working solution for extracting real StarCraft: Remastered sprites with authentic colors and all animation frames.

## Architecture

### Component Stack
```
┌─────────────────────────────────────┐
│  StarCraft: Remastered              │
│  CASC Archive (/Applications/SC)    │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│  CascLib (C Library)                │
│  - CascOpenStorage()                │
│  - CascOpenFile()                   │
│  - CascReadFile()                   │
└──────────────┬──────────────────────┘
               │ FFI Bindings
┌──────────────▼──────────────────────┐
│  Rust Extractor                     │
│  - casclib_ffi.rs (FFI)            │
│  - grp/mod.rs (Parser)             │
│  - palette.rs (Real colors)        │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│  PNG Export                         │
│  - Single frames                    │
│  - Sprite sheets (17 frames/row)   │
└─────────────────────────────────────┘
```

## GRP File Format

### Header (6 bytes)
```rust
frame_count: u16  // Number of animation frames
width: u16        // Frame width in pixels
height: u16       // Frame height in pixels
```

### Frame Table (8 bytes per frame)
```rust
x_offset: u8      // Horizontal offset
y_offset: u8      // Vertical offset
unknown: u16      // Unknown (possibly flags)
file_offset: u32  // Offset to frame data
```

**Key Discovery**: Frame offsets can be duplicated (same offset for multiple frames) for animation purposes. Must find next *different* offset to determine frame data size.

### Frame Data Structure
```rust
// Line offset table
first_offset: u16  // Offset to first line data
// ... more offsets (line_count = first_offset / 2)

// Line data (RLE encoded)
// Decoded right-to-left, bottom-to-top
```

### RLE Encoding
```rust
if byte >= 0x80 {
    // Transparent pixels
    skip_count = byte - 0x80;
    x -= skip_count;
}
else if byte > 0x40 {
    // Run-length encoding
    count = byte - 0x40;
    pixel_value = next_byte;
    repeat pixel_value 'count' times;
}
else {
    // Literal pixels
    count = byte;
    copy next 'count' bytes;
}
```

## Palette Format

### Source
Extracted from `game\tunit.pcx` in CASC archive.

### PCX Palette Structure
```
File size - 769 bytes:
  [0]: 0x0C (palette marker)
  [1-768]: RGB triplets (256 colors × 3 bytes)
```

### Palette Regions
- **0**: Transparent (alpha = 0)
- **1-7**: Shadows and dark colors
- **8-15**: Player color regions (remappable)
- **16-255**: Standard game colors

### Player Colors
Indices 8-15 appear as pink/purple in extracted sprites. These get replaced with team colors during gameplay:
- Red, Blue, Teal, Purple, Orange, Brown, White, Yellow

## CascLib FFI

### Library Setup
```bash
# Build CascLib
cd /tmp/CascLib
mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=ON
make -j8

# Copy to project
cp casc.framework/casc tools/casc-extractor/lib/libcasc.dylib
```

### Rust Bindings
```rust
type Handle = *mut c_void;
type DWORD = u32;
type BOOL = i32;

extern "C" {
    fn CascOpenStorage(
        path: *const c_char,
        locale: DWORD,
        storage: *mut Handle
    ) -> BOOL;
    
    fn CascOpenFile(
        storage: Handle,
        filename: *const c_char,
        locale: DWORD,
        flags: DWORD,
        file: *mut Handle
    ) -> BOOL;
    
    fn CascReadFile(
        file: Handle,
        buffer: *mut c_void,
        size: DWORD,
        read: *mut DWORD
    ) -> BOOL;
}
```

### Usage
```rust
let archive = CascArchive::open("/Applications/StarCraft")?;
let data = archive.extract_file("unit\\terran\\marine.grp")?;
```

## Sprite Sheet Layout

### Standard Layout
- **Frames per row**: 17 (StarCraft standard)
- **Row count**: ceil(frame_count / 17)
- **Sheet width**: frame_width × 17
- **Sheet height**: frame_height × rows

### Animation Structure
StarCraft sprites typically contain:
- 8 directional walking (N, NE, E, SE, S, SW, W, NW)
- Attack animations per direction
- Death animations
- Idle animations

### Example: Marine
- **Frames**: 229
- **Layout**: 17 × 14 rows
- **Sheet size**: 1088 × 896 pixels
- **Frame size**: 64 × 64 pixels

## Build Configuration

### Cargo.toml Dependencies
```toml
[dependencies]
png = "0.17"
byteorder = "1.5"
log = "0.4"
env_logger = "0.11"
```

### build.rs
```rust
fn main() {
    println!("cargo:rustc-link-search=native=lib");
    println!("cargo:rustc-link-lib=dylib=casc");
    println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path/../lib");
}
```

### Runtime
```bash
DYLD_LIBRARY_PATH=lib cargo run --release --bin extract_all_units
```

## Known File Paths

### Units
```
unit\terran\marine.grp
unit\terran\firebat.grp
unit\protoss\zealot.grp
unit\protoss\dragoon.grp
unit\zerg\zergling.grp
unit\zerg\hydralisk.grp
```

### Palette
```
game\tunit.pcx  (contains 256-color palette)
```

## Performance

### Extraction Speed
- **Single unit**: ~50ms
- **25 units**: ~1.5 seconds
- **Sprite sheet generation**: ~200ms per unit

### Output Sizes
- **Single frame PNG**: 1-7 KB
- **Sprite sheet PNG**: 50-500 KB (depends on frame count)

## Troubleshooting

### Library Not Found
```bash
# Check library exists
ls -la lib/libcasc.dylib

# Check library ID
otool -L lib/libcasc.dylib

# Fix if needed
install_name_tool -id @rpath/libcasc.dylib lib/libcasc.dylib
```

### Parsing Errors
Some units use format variants:
- High Templar, Archon, Arbiter have different frame structures
- Can be fixed by analyzing their specific format

### Color Issues
Ensure using real palette from `src/palette.rs` (extracted from game files), not generated palette.

## References

- **GRP Format**: `https://hwiegman.home.xs4all.nl/fileformats/G/grp/GRP.txt`
- **CascLib**: `https://github.com/ladislav-zezula/CascLib`
- **StarCraft Modding**: `https://github.com/poiuyqwert/PyMS`

---

**Last Updated**: 2026-01-25  
**Maintainer**: StarCraft: Reimagined Team
