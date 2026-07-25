# StarCraft Sprites - Unity Integration Guide

## Overview

`extract anim --convert-to-png` produces everything Unity needs:
- **PNG sprite sheets** — the diffuse layer, frames packed into an atlas
- **JSON metadata** — per-frame rects for automatic slicing
- **Unity Editor scripts** — in this repo's [`unity/`](../unity/) directory

Up to ~1000 HD sprites or 868 SD sprites, depending on quality; see
[Sprite Reference](sprite-reference.md).

## File Structure

Names come from the built-in mapping, so output mirrors its keys:

```bash
casc-extractor extract anim --quality hd4 --convert-to-png --output extracted
```

```
extracted/
├── terran/marine.png      # Sprite sheet (diffuse layer)
├── terran/marine.json     # Frame metadata for slicing
├── zerg/guardian.png
└── zerg/guardian.json
```

Add `--team-color-mask` for a `_tc.png` mask beside each sheet, or `--raw` for
numbered `main_NNN.png` filenames.

## Unity Integration (3 Steps)

### Step 1: Copy Sprites to Unity

1. Copy the entire `output/` folder to your Unity project:
   ```
   Assets/StarCraftSprites/
   ```

2. The folder structure will be preserved:
   ```
   Assets/StarCraftSprites/
   ├── terran/
   ├── protoss/
   ├── zerg/
   ├── effects/
   ├── neutral/
   └── ui/
   ```

### Step 2: Install Unity Editor Scripts

1. Create an `Assets/Editor/` folder if it doesn't exist

2. Copy both scripts from this repo's [`unity/`](../unity/) directory to
   `Assets/Editor/`:
   - `StarcraftSpriteImporter.cs` — automatic sprite slicing on import
   - `StarcraftAnimationCreator.cs` — one-click animation creation

3. Unity will automatically compile the scripts

### Step 3: Import and Use

**Automatic Import (Recommended):**
- Sprites are automatically sliced when imported
- JSON metadata is read and applied
- Each frame becomes a separate sprite asset

**Manual Animation Creation:**
1. Select sprite sheets in Project window
2. Go to `Tools > StarCraft > Create Animations from Selected Sprites`
3. Animations are created in the same folder

## JSON Metadata Format

Each sprite has a JSON file alongside its PNG. Real output for `main_025`:

```json
{
  "name": "main_025",
  "frameCount": 119,
  "grpWidth": 421,
  "grpHeight": 420,
  "textureWidth": 4072,
  "textureHeight": 1560,
  "pixelsPerUnit": 100.0,
  "frames": [
    {"index": 0, "x": 3208, "y": 1408, "width": 268, "height": 140,
     "offsetX": 79, "offsetY": 101}
  ]
}
```

Two things to know when writing your own importer:

- **Frames are not a uniform grid.** ANIM frames are trimmed and packed, so each has
  its own size and position. There is no `frameWidth`/`framesPerRow` to divide by.
- **`y` is top-down**; Unity's texture space is bottom-up, so flip with
  `textureHeight - y - height`. `offsetX`/`offsetY` place the trimmed frame within
  the logical `grpWidth`×`grpHeight` bounds — ignore them and animations jitter.

`pixelsPerUnit` comes from `output_settings.unity_settings.pixels_per_unit` in the
config file.

## Import Settings

The automatic importer configures:
- **Texture Type**: Sprite (2D and UI)
- **Sprite Mode**: Multiple
- **Pixels Per Unit**: Matches frame size
- **Filter Mode**: Point (pixel-perfect)
- **Compression**: None (preserve quality)
- **Alpha**: Transparency enabled
- **Pivot**: Bottom-center (perfect for RTS units)

## Animation Settings

Animations are created with:
- **Frame Rate**: 15 FPS (StarCraft standard)
- **Loop**: Enabled
- **Naming**: `{sprite_name}_anim.anim`

## Usage in Game

### Basic Sprite Rendering

```csharp
// Get sprite from sheet
Sprite[] sprites = Resources.LoadAll<Sprite>("StarCraftSprites/terran/units/marine");
SpriteRenderer renderer = GetComponent<SpriteRenderer>();
renderer.sprite = sprites[0]; // First frame
```

### Using Animations

```csharp
// Load and play animation
AnimationClip clip = Resources.Load<AnimationClip>("StarCraftSprites/terran/units/marine_anim");
Animator animator = GetComponent<Animator>();
animator.Play(clip.name);
```

### ECS Integration (Unity 6.2)

```csharp
// Convert sprite to mesh for Entities Graphics
public struct UnitRenderComponent : IComponentData
{
    public UnityObjectRef<Mesh> SpriteMesh;
    public UnityObjectRef<Material> SpriteMaterial;
}

// Baker for sprite conversion
public class UnitSpriteBaker : Baker<UnitAuthoring>
{
    public override void Bake(UnitAuthoring authoring)
    {
        var entity = GetEntity(TransformUsageFlags.Dynamic);
        
        // Convert sprite to mesh
        var mesh = SpriteToMeshConverter.CreateMeshFromSprite(authoring.sprite);
        var material = SpriteToMeshConverter.CreateSpriteMaterial(authoring.sprite);
        
        AddComponent(entity, new UnitRenderComponent
        {
            SpriteMesh = mesh,
            SpriteMaterial = material
        });
    }
}
```

## Sprite Categories

The built-in mapping covers units and buildings for all three races plus effects and
scenery (flames, blood, selection circles, vespene smoke, mineral fields, explosions).
It is a flat name-to-id table rather than a category tree — see
[Sprite Reference](sprite-reference.md).

## Performance Tips

1. **Use Sprite Atlasing**: Unity will automatically batch sprites from the same atlas
2. **Point Filtering**: Preserves pixel art quality
3. **No Mipmaps**: Saves memory for 2D sprites
4. **Uncompressed**: Best quality for retro sprites
5. **Bottom-Center Pivot**: Perfect for RTS ground units

## Troubleshooting

**Sprites not slicing automatically?**
- Ensure folder is named `StarCraftSprites`
- Check that JSON files are present
- Reimport the texture (right-click > Reimport)

**Animations not playing?**
- Verify Animator component is attached
- Check animation clip is assigned to Animator Controller
- Ensure frame rate matches (15 FPS default)

**Sprites look blurry?**
- Change Filter Mode to Point
- Disable mipmaps
- Use uncompressed format

## Advanced: Custom Animation States

Create an Animator Controller with multiple animations:

```csharp
// Example: Marine with idle, walk, attack animations
// 1. Create Animator Controller
// 2. Add animation clips as states
// 3. Create transitions between states
// 4. Control via code:

animator.SetTrigger("Walk");
animator.SetTrigger("Attack");
animator.SetTrigger("Idle");
```

## Complete Asset List

See [Sprite Reference](sprite-reference.md) for how sprites are addressed and named,
and `casc-extractor inspect sprites` for what your archive actually contains.

## Support

For issues or questions:
1. Check JSON metadata is valid
2. Verify Unity Editor scripts are in `Assets/Editor/`
3. Ensure Unity version is 2021.3 or later
4. Check console for import errors

---

All sprites are ready for production use in your Unity project.
