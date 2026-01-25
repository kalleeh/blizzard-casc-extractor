# Blizzard CASC Sprite Extractor

Extract sprites from Blizzard games using CASC archives with authentic colors and complete animation frames.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Supported Games

- ✅ **StarCraft: Remastered** - Fully tested (133 sprites)
- 🔧 **Warcraft III: Reforged** - CASC compatible (untested)
- 🔧 **Heroes of the Storm** - CASC compatible (untested)
- 🔧 **World of Warcraft** - CASC compatible (untested)
- 🔧 **Diablo III** - CASC compatible (untested)
- 🔧 **Overwatch** - CASC compatible (untested)

*Currently optimized for StarCraft: Remastered. Other games may require format-specific parsers.*

## Features

- ✅ **Complete sprite extraction** - All game sprites (units, buildings, effects)
- ✅ **Authentic colors** - Real game palettes from game files
- ✅ **Full animations** - All animation frames preserved
- ✅ **Unity integration** - JSON metadata for automatic sprite slicing
- ✅ **Organized output** - Structured folders by type
- ✅ **Extensible** - Add parsers for other Blizzard formats

## StarCraft: Remastered Results

- **44 units** (Terran, Protoss, Zerg)
- **46 buildings** (all structures)
- **33 effects** (projectiles, explosions, fire, smoke, blood)
- **7 neutral** (critters, resources)
- **3 UI** (wireframes)

## Requirements

- Rust 1.70+
- A Blizzard game using CASC (StarCraft: Remastered, WC3: Reforged, etc.)
- macOS (ARM64) or Linux (x86_64)
- CascLib (included)

## Quick Start

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
extracted/organized/
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
- `.png` - Sprite sheet with all animation frames
- `.json` - Unity metadata for automatic slicing
- `.txt` - Human-readable information

## Unity Integration

Automatic sprite slicing and animation creation:

1. Copy sprites to `Assets/YourFolder/`
2. Copy `unity/*.cs` to `Assets/Editor/`
3. Sprites auto-slice on import
4. Create animations: Tools > StarCraft > Create Animations

See [UNITY_INTEGRATION.md](UNITY_INTEGRATION.md) for details.

## Technical Details

### Architecture

- **CascLib FFI** - Rust bindings to CascLib for CASC archive access
- **GRP Parser** - Custom parser for StarCraft's GRP sprite format
- **RLE Decoder** - Handles run-length encoded sprite data
- **Palette System** - Authentic 256-color StarCraft palette

### GRP Format

StarCraft sprites use the GRP format:
- Header: frame count, width, height
- Frame table: 8 bytes per frame (offsets, dimensions)
- Frame data: RLE-encoded pixel data

See [TECHNICAL_REFERENCE.md](TECHNICAL_REFERENCE.md) for implementation details.

## Documentation

- [Getting Started](docs/getting-started.md) - Complete setup guide
- [Sprite Reference](docs/sprite-reference.md) - List of extractable sprites
- [Unity Integration](docs/unity-guide.md) - Unity setup and usage
- [Technical Guide](docs/technical-guide.md) - Implementation details

## License

MIT License - See [LICENSE](LICENSE) for details.

## Legal Notice

This tool extracts sprites from Blizzard games using CASC archives for personal use and game development learning. You must own a legal copy of the game to use this tool. Extracted sprites and game assets are property of Blizzard Entertainment.

StarCraft®, Warcraft®, World of Warcraft®, Diablo®, Heroes of the Storm®, and Overwatch® are registered trademarks of Blizzard Entertainment, Inc.

This project is not affiliated with, endorsed by, or sponsored by Blizzard Entertainment, Inc.

## Contributing

Contributions welcome! Especially:
- **Format parsers** for other Blizzard games (WC3, HotS, WoW, etc.)
- **Platform support** - Windows, Linux builds
- **Additional formats** - PCX, WAV, BLP, M2, etc.
- **Documentation** - Game-specific extraction guides

See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

Contributions welcome! Please:
1. Fork the repository
2. Create a feature branch
3. Add tests for new features
4. Submit a pull request

## Acknowledgments

- [CascLib](https://github.com/ladislav-zezula/CascLib) by Ladislav Zezula
- StarCraft modding community

---

**A tool for extracting sprites from Blizzard CASC archives.**
