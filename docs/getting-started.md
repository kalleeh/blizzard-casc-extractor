# Getting Started

Complete setup guide for extracting sprites from Blizzard CASC archives.

## Prerequisites

- **Rust 1.70+** - [Install Rust](https://rustup.rs/)
- **Blizzard Game** - StarCraft: Remastered, Warcraft III: Reforged, etc.
- **Git** - For cloning the repository
- **CMake** - For building CascLib

## Installation

### 1. Clone Repository

```bash
git clone https://github.com/kalleeh/blizzard-casc-extractor
cd blizzard-casc-extractor
```

### 2. Build CascLib

CascLib is required to read CASC archives.

**macOS/Linux:**
```bash
cd /tmp
git clone https://github.com/ladislav-zezula/CascLib
cd CascLib
mkdir build && cd build
cmake .. -DCASC_BUILD_SHARED_LIB=ON
make

# Copy library to project
cp libcasc.* /path/to/blizzard-casc-extractor/lib/
```

**Verify:**
```bash
ls lib/
# Should show: libcasc.dylib (macOS) or libcasc.so (Linux)
```

### 3. Build Project

```bash
cargo build --release
```

## Usage

### Extract All Sprites

```bash
# All HD sprites, named from the built-in mapping, converted to PNG
./target/release/casc-extractor extract anim --quality hd4 --convert-to-png

# All SD sprites (868 PNGs from the single mainSD.anim)
./target/release/casc-extractor extract anim --quality sd --convert-to-png
```

CascLib is resolved relative to the executable, so no `DYLD_LIBRARY_PATH` /
`LD_LIBRARY_PATH` is needed.

### Output

Sprites are extracted to `output/`:

```
output/
├── terran/
│   ├── units/
│   │   ├── marine.png
│   │   ├── marine.json
│   │   └── marine.txt
│   └── buildings/
├── protoss/
├── zerg/
├── effects/
├── neutral/
└── ui/
```

Each sprite includes:
- `.png` - Sprite sheet with all animation frames
- `.json` - Unity metadata for automatic slicing
- `.txt` - Human-readable information

## Configuration

### Change Game Path

The installation is auto-detected. Override it with a flag — no code edits:

```bash
./target/release/casc-extractor --install-path /path/to/StarCraft inspect archive
```

### Customize Mappings

Edit `mappings/starcraft-remastered.yaml` to add/remove sprites:

```yaml
terran/units/marine: unit\terran\marine.grp
protoss/units/zealot: unit\protoss\zealot.grp
```

## Troubleshooting

### "Library not loaded: libcasc.dylib"

The binary finds CascLib via a relative rpath, so this means `lib/` is not where
it expects. Build CascLib into `./lib` (see `lib/README.md`) and run the binary
from `target/release/`; if you copied the binary elsewhere, copy `lib/` beside it.
As a last resort, point the loader at it explicitly:

```bash
export DYLD_LIBRARY_PATH=lib DYLD_FRAMEWORK_PATH=lib  # macOS
export LD_LIBRARY_PATH=lib                            # Linux
```

### "Failed to open CASC storage"

- Verify the game is installed
- Pass `--install-path` if auto-detection picked the wrong directory
- Ensure you have read permissions

### "No sprites extracted"

- Check `mappings/` files for correct paths
- Verify game version (tested with StarCraft: Remastered 1.23+)

## Next Steps

- [Unity Integration](unity-guide.md) - Import sprites into Unity
- [Technical Guide](technical-guide.md) - Understand the implementation
- [Extraction Results](extraction-results.md) - See what's extracted

## Quick Reference

```bash
# Build
cargo build --release

# Extract
./target/release/casc-extractor extract anim --quality hd4 --convert-to-png

# Test
cargo test --lib && cargo test --tests

# Clean
cargo clean
rm -rf output/
```
