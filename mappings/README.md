# Sprite Extraction Mapping System

## Overview

Configurable sprite extraction using YAML mapping files. Supports organized output with proper folder structure and metadata.

## Usage

```bash
cd tools/casc-extractor
DYLD_LIBRARY_PATH=lib cargo run --release --bin extract_organized
```

## Mapping Format

```yaml
# Format: category/subcategory/name: file_path_in_archive
terran/units/marine: unit\terran\marine.grp
terran/buildings/barracks: unit\terran\barracks.grp
protoss/units/zealot: unit\protoss\zealot.grp
```

## Output Structure

```
extracted/organized/
├── terran/
│   ├── units/
│   │   ├── marine.png (sprite sheet)
│   │   ├── marine.txt (metadata)
│   │   └── ...
│   └── buildings/
│       ├── barracks.png
│       └── ...
├── protoss/
│   ├── units/
│   └── buildings/
└── zerg/
    ├── units/
    └── buildings/
```

## Metadata Files

Each sprite includes a `.txt` file with:
- Frame count
- Frame dimensions
- Sheet dimensions
- Layout (frames per row × rows)

Example `marine.txt`:
```
frames: 229
frame_size: 64x64
sheet_size: 1088x896
layout: 17x14
```

## Current Results

**38 sprites successfully extracted:**
- Terran: 12 sprites (units + buildings)
- Protoss: 14 sprites (units + buildings)
- Zerg: 12 sprites (units + buildings)

## Adding New Games

Create a new mapping file in `mappings/`:

```yaml
# mappings/warcraft3.yaml
human/units/footman: Units\Human\Footman.mdx
orc/units/grunt: Units\Orc\Grunt.mdx
```

Then modify the extraction tool to use the new mapping.

## Known Issues

Some sprites have parsing errors (different GRP format variants):
- Buildings: Factory, Starport, Control Tower, Hatchery, Lair, Hive, Spire
- Units: High Templar, Archon, Arbiter

These use slightly different frame table structures and need format-specific parsers.

## Extending the System

### Add New Categories

```yaml
effects/explosions/small: unit\thingy\explosion1.grp
ui/buttons/attack: rez\buttons\attack.grp
```

### Add Alternate Paths

The system tries each path in order:

```yaml
terran/units/wraith: 
  - unit\terran\wraith.grp
  - unit\terran\twraith.grp
  - SD\unit\terran\wraith.grp
```

(Note: Multi-path support requires code modification)

## Performance

- **Single sprite**: ~50ms extraction + conversion
- **38 sprites**: ~2 seconds total
- **Output size**: 1-500KB per sprite sheet (depends on frame count)
