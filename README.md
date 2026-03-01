# Blizzard CASC Sprite Extractor

Extract sprites from Blizzard games using CASC archives with authentic colors and complete animation frames.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Supported Games

**StarCraft: Remastered** - Fully tested (133 sprites extracted)

Other CASC-based games are compatible but untested:
- Warcraft III: Reforged
- Heroes of the Storm
- World of Warcraft
- Diablo III
- Overwatch

*Note: Other games may require format-specific parsers.*

## Features

- **HD Content Extraction** - 4x Ultra HD and 2x HD animated sprites
- **Quality Level Selection** - Extract SD, HD2, or HD4 content
- **ANIM Format Parser** - Parse HD animated sprites with multiple layers
- **Batch Extraction** - Extract animations, tilesets, and effects
- Complete sprite extraction (units, buildings, effects)
- Authentic game palettes from original files
- Full animation frame preservation
- Unity integration with JSON metadata
- Organized output structure
- Extensible parser architecture

## Extraction Results

StarCraft: Remastered includes:
- 44 units (Terran, Protoss, Zerg)
- 46 buildings (all structures)
- 33 effects (projectiles, explosions, fire, smoke, blood)
- 7 neutral (critters, resources)
- 3 UI elements (wireframes)

### HD Content Quality Levels

StarCraft: Remastered provides **three quality levels** of assets:

#### 4x HD (Ultra - 4K Quality)
- **Path**: `anim/`, `tileset/`, `effect/` (NO PREFIX)
- **Animations**: ~4.5 MB per sprite (999 sprites available)
- **Tilesets**: ~55 MB per tileset (8 tilesets)
- **Effects**: Water, fire, explosions in 4K
- **Best for**: High-resolution displays, modern Unity projects

#### 2x HD
- **Path**: `HD2/anim/`, `HD2/tileset/`, `HD2/effect/`
- **Animations**: ~1.1 MB per sprite
- **Tilesets**: ~14 MB per tileset
- **Best for**: Balanced quality/performance

#### SD (Original)
- **Path**: `SD/mainSD.anim` (single 38 MB file with all sprites)
- **Format**: Paletted 256-color graphics
- **Best for**: Authentic retro look, minimal file size

**Total HD Content**: ~5.2 GB in `Data/data/` folder

## Requirements

- Rust 1.70+
- A Blizzard game using CASC (StarCraft: Remastered, WC3: Reforged, etc.)
- macOS (ARM64) or Linux (x86_64)
- CascLib native library (must be built from source — see `lib/README.md`)

## Quick Start

### Extract HD Content (NEW!)

```bash
# Extract 4x Ultra HD animations, tilesets, and effects
cd tools/casc-extractor
cargo run --release --bin extract_hd -- --quality hd4 --all

# Extract specific quality level
cargo run --release --bin extract_hd -- --quality hd2 --animations

# Extract specific animations by ID
cargo run --release --bin extract_hd -- --anim-ids 0,1,7 --animations

# Available quality levels:
# - sd:  Original quality (GRP format)
# - hd2: 2x HD (1.1MB animations, 14MB tilesets)
# - hd4: 4x Ultra HD (4.5MB animations, 55MB tilesets) [DEFAULT]
```

### Extract SD Sprites (Legacy)

```bash
# Clone repository
git clone https://github.com/kalleeh/blizzard-casc-extractor
cd blizzard-casc-extractor

# Build CascLib
cd /tmp
git clone https://github.com/ladislav-zezula/CascLib
cd CascLib
mkdir build && cd build
cmake .. -DCASC_BUILD_SHARED_LIB=ON
make
cp libcasc.* /path/to/casc-extractor/lib/

# Build and run
cargo build --release
DYLD_LIBRARY_PATH=lib ./target/release/extract_organized
```

## Output Structure

```
output/
├── terran/
│   ├── units/          # Marine, Firebat, Ghost, etc.
│   └── buildings/      # Command Center, Barracks, etc.
├── protoss/
│   ├── units/          # Probe, Zealot, Dragoon, etc.
│   └── buildings/      # Nexus, Gateway, etc.
├── zerg/
│   ├── units/          # Drone, Zergling, Hydralisk, etc.
│   └── buildings/      # Hatchery, Spawning Pool, etc.
├── effects/            # Projectiles, explosions, etc.
├── neutral/            # Critters, resources
└── ui/                 # Wireframes
```

Each sprite includes:
- PNG sprite sheet with all animation frames
- JSON metadata for Unity automatic slicing
- Text file with human-readable information

## Unity Integration

The extractor includes Unity Editor scripts for automatic sprite slicing and animation creation:

1. Copy extracted sprites to your Unity project
2. Copy `unity/*.cs` scripts to `Assets/Editor/`
3. Sprites will auto-slice on import
4. Create animations via Tools > StarCraft > Create Animations

See [Unity Integration Guide](docs/unity-guide.md) for complete setup instructions.

## Technical Details

### Architecture

The extractor uses:
- **CascLib FFI** - Rust bindings for CASC archive access
- **GRP Parser** - Custom parser for StarCraft's sprite format
- **RLE Decoder** - Run-length encoded sprite data handling
- **Palette System** - Authentic 256-color game palette

### GRP Format

StarCraft sprites use the GRP format with:
- Header containing frame count, width, and height
- Frame table with 8 bytes per frame (offsets, dimensions)
- RLE-encoded pixel data for each frame

See [Technical Guide](docs/technical-guide.md) for implementation details.

## Documentation

- [Getting Started](docs/getting-started.md) - Installation and setup
- [Sprite Reference](docs/sprite-reference.md) - Available sprites
- [Unity Integration](docs/unity-guide.md) - Unity workflow
- [Technical Guide](docs/technical-guide.md) - Implementation details

## License

MIT License - See [LICENSE](LICENSE) for details.

## Legal Notice

This tool extracts sprites from Blizzard games using CASC archives for personal use and game development learning. You must own a legal copy of the game to use this tool. Extracted sprites and game assets are property of Blizzard Entertainment.

StarCraft®, Warcraft®, World of Warcraft®, Diablo®, Heroes of the Storm®, and Overwatch® are registered trademarks of Blizzard Entertainment, Inc.

This project is not affiliated with, endorsed by, or sponsored by Blizzard Entertainment, Inc.

## Contributing

Contributions are welcome, especially:
- Format parsers for other Blizzard games
- Windows and Linux build support
- Additional format support (PCX, WAV, BLP, M2)
- Game-specific extraction guides

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Acknowledgments

- [CascLib](https://github.com/ladislav-zezula/CascLib) by Ladislav Zezula
- StarCraft sprite format research by the modding community

- [CascLib](https://github.com/ladislav-zezula/CascLib) by Ladislav Zezula
- StarCraft modding community

---

**A tool for extracting sprites from Blizzard CASC archives.**
