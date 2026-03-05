# SC:R HD Sprite Team Color Compositing

## HD ANIM Layer Structure

Every HD ANIM file contains up to 7 named layers stored in order:

| Index | Name       | Format | Description                                      |
|-------|------------|--------|--------------------------------------------------|
| 0     | `diffuse`  | DXT5   | Full-colour base texture                         |
| 1     | `bright`   | DXT5   | Lit/highlight pass (not team-coloured)           |
| 2     | `teamcolor`| DXT1   | Binary mask — white = team-colour region         |
| 3     | `emissive` | DXT5   | Glowing/emissive pixels (often empty)            |
| 4     | `normal`   | DXT5   | Normal map                                       |
| 5     | `specular` | DXT1   | Specular mask                                    |
| 6     | `ao_depth` | DXT5   | Ambient occlusion / depth                        |

**Important:** the `teamcolor` layer at index 2 is a **DXT1 binary mask** — its pixels
are either fully opaque white (team-colour region) or fully transparent (not team-coloured).
It is *not* the same as the derived `_tc.png` that `--team-color-mask` exports (which is
derived from the `bright` layer and has varying luminance).

The layer *names* are always present in the header, but the layer *data* may be empty
(`size == 0`) for sprites that carry no team colour (neutral units, effects, etc.).

## Correct Compositing Formula

Source: reverse-engineered from Blizzard's SC:R D3D11 shader
(reference: `neivv/mtl` — `src/shaders/d3d11/sprite_part_solid_frag.hlsl`).

```
for each pixel (x, y):
    if teamcolor_mask[x,y].alpha > 0:
        output[x,y].rgb = diffuse[x,y].rgb  *  (player_color.rgb / 255)
        output[x,y].a   = diffuse[x,y].a
    else:
        output[x,y] = diffuse[x,y]          # unchanged
```

In compact form:  `output.rgb = diffuse.rgb * player.rgb`  for masked pixels.

### Why it works

The `diffuse` layer stores **neutral grey values** in the team-colour regions
(measured saturation: 0.06–0.14 in the grey channel).  Multiplying a grey pixel
by a vivid player colour produces a shaded, brightness-preserving tint:

- dark grey × blue  →  dark blue  (shadow areas)
- mid grey  × blue  →  medium blue
- light grey × blue →  bright blue (highlight areas)

The non–team-colour parts of the diffuse (yellow Protoss hull, brown Zerg
carapace, etc.) remain **completely unchanged**.

## SC1 / SC:R Player Colours

| Player | Colour  | Hex      | RGB           |
|--------|---------|----------|---------------|
| 1      | Red     | #F40404  | 244, 4, 4     |
| 2      | Blue    | #0C48CC  | 12, 72, 204   |
| 3      | Teal    | #2CB494  | 44, 180, 148  |
| 4      | Purple  | #88409C  | 136, 64, 156  |
| 5      | Orange  | #F88C14  | 248, 140, 20  |
| 6      | Brown   | #703014  | 112, 48, 20   |
| 7      | White   | #CCE0D0  | 204, 224, 208 |
| 8      | Yellow  | #FCFC38  | 252, 252, 56  |

Player 8 yellow (`#FCFC38`) produces an output that closely matches the default
diffuse — useful for validation that compositing is correct.

## Reading the Layer from the ANIM File

```
Header layout (bytes from file start):
  0..4   magic      "ANIM"
  4..6   version    u16  (0x0202 or 0x0204)
  6..8   unknown    u16
  8..10  layer_count u16
  10..12 entry_count u16
  12..332 layer_names  10 × 32-byte null-padded strings

Entry layout (starts at byte 332):
  0..2   frame_count u16
  2..4   unknown     u16
  4..6   grp_width   u16
  6..8   grp_height  u16
  8..12  frame_ptr   u32
  12..132 images     10 × (ptr u32, size u32, tex_width u16, tex_height u16)

The 'teamcolor' layer name is at position 2 in the names list.
The corresponding image entry is images[2].
If images[2].size == 0 the sprite has no team colour.
```

## Python Reference Implementation

```python
import struct, subprocess
from PIL import Image

def get_tc_layer_dds(anim_bytes):
    """Return raw DDS bytes of the teamcolor layer, or None if absent."""
    names = [anim_bytes[12+i*32:12+i*32+32].rstrip(b'\x00').decode('latin-1')
             for i in range(10)]
    try:
        tc_idx = names.index('teamcolor')
    except ValueError:
        return None
    img_off = 332 + 12 + tc_idx * 12     # entry header + image slot
    ptr, size = struct.unpack_from('<II', anim_bytes, img_off)
    if size == 0 or ptr == 0:
        return None
    return anim_bytes[ptr:ptr+size]

def apply_player_color(diffuse_png, tc_dds_bytes, player_rgb):
    """
    Composite player_rgb onto diffuse using the binary teamcolor mask.
    Returns a PIL RGBA Image.
    """
    # Decode DXT1 teamcolor mask via ImageMagick
    with open('/tmp/_tc.dds', 'wb') as f:
        f.write(tc_dds_bytes)
    subprocess.run(['magick', '/tmp/_tc.dds', '/tmp/_tc.png'], check=True)

    diffuse = Image.open(diffuse_png).convert('RGBA')
    tc      = Image.open('/tmp/_tc.png').convert('RGBA')
    d, m    = diffuse.load(), tc.load()
    w, h    = diffuse.size
    result  = Image.new('RGBA', (w, h))
    out     = result.load()
    pr, pg, pb = [c / 255.0 for c in player_rgb]

    for y in range(h):
        for x in range(w):
            dr, dg, db, da = d[x, y]
            mr, mg, mb, ma = m[x, y]
            if ma > 0 and da > 0:
                out[x, y] = (int(dr * pr), int(dg * pg), int(db * pb), da)
            else:
                out[x, y] = (dr, dg, db, da)
    return result
```

## Validated Sprites

Tested on HD4 quality extracts from the SC:R archive:

| ANIM ID | Unit           | Has TC layer | Notes                        |
|---------|----------------|:------------:|------------------------------|
| 000     | Zerg Broodling | ✓            | Blue wing membranes          |
| 004     | Effect splash  | ✓            | Teal colouring works         |
| 025     | Zergling       | ✓            | Blue accent scales           |
| 118     | Protoss unit   | ✓            | Gold hull unchanged, details coloured |
| 250     | Terran vehicle | ✓            | Red accent panels            |
| 001     | Unit           | ✗            | No TC data (neutral sprite)  |
| 047     | Unit           | ✗            | No TC data                   |
| 135     | Unit           | ✗            | No TC data                   |
