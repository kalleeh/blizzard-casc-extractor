# StarCraft Sprites - Unity Integration Guide

## Overview

All 133 StarCraft sprites have been extracted with complete Unity integration metadata:
- **PNG sprite sheets** - All animation frames in grid layout
- **JSON metadata** - Frame positions and dimensions for automatic slicing
- **TXT metadata** - Human-readable sprite information
- **Unity Editor scripts** - Automatic import and animation creation

## File Structure

```
extracted/organized/
├── terran/units/marine.png          # Sprite sheet
├── terran/units/marine.json         # Unity metadata
├── terran/units/marine.txt          # Human-readable info
└── ... (133 sprites total)
```

## Unity Integration (3 Steps)

### Step 1: Copy Sprites to Unity

1. Copy the entire `extracted/organized/` folder to your Unity project:
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

2. Copy these scripts to `Assets/Editor/`:
   - `StarcraftSpriteImporter.cs` - Automatic sprite slicing on import
   - `StarcraftAnimationCreator.cs` - One-click animation creation

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

Each sprite has a JSON file with this structure:

```json
{
  "frameCount": 229,
  "frameWidth": 64,
  "frameHeight": 64,
  "framesPerRow": 17,
  "rows": 14,
  "sheetWidth": 1088,
  "sheetHeight": 896,
  "frames": [
    {"index": 0, "x": 0, "y": 0, "width": 64, "height": 64},
    {"index": 1, "x": 64, "y": 0, "width": 64, "height": 64},
    ...
  ]
}
```

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

### Units (44 sprites)
- **Terran**: 13 units (Marine, Firebat, Ghost, etc.)
- **Protoss**: 15 units (Probe, Zealot, Dragoon, etc.)
- **Zerg**: 16 units (Drone, Zergling, Hydralisk, etc.)

### Buildings (46 sprites)
- **Terran**: 15 buildings
- **Protoss**: 16 buildings
- **Zerg**: 15 buildings

### Effects (33 sprites)
- Projectiles (18)
- Explosions (3)
- Fire (3)
- Smoke (2)
- Blood (4)
- Other (3)

### Neutral (7 sprites)
- Vespene Geyser
- Critters (6)

### UI (3 sprites)
- Wireframes (Terran, Protoss, Zerg)

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

See `FINAL_COMPLETE.md` for the complete list of all 133 extracted sprites.

## Support

For issues or questions:
1. Check JSON metadata is valid
2. Verify Unity Editor scripts are in `Assets/Editor/`
3. Ensure Unity version is 2021.3 or later
4. Check console for import errors

---

**All sprites ready for production use in your StarCraft: Reimagined Unity project!** 🚀
