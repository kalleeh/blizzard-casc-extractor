using System.Collections.Generic;
using System.IO;
using UnityEditor;
using UnityEngine;

/// <summary>
/// Automatically imports and slices StarCraft sprite sheets based on JSON metadata.
/// Place this script in Assets/Editor/ folder.
/// </summary>
public class StarcraftSpriteImporter : AssetPostprocessor
{
    void OnPreprocessTexture()
    {
        // Only process PNG files in the StarCraft sprites folder
        if (!assetPath.Contains("StarCraftSprites") || !assetPath.EndsWith(".png"))
            return;

        // Check if JSON metadata exists
        string jsonPath = assetPath.Replace(".png", ".json");
        if (!File.Exists(jsonPath))
            return;

        TextureImporter importer = (TextureImporter)assetImporter;
        
        // Configure texture import settings
        importer.textureType = TextureImporterType.Sprite;
        importer.spriteImportMode = SpriteImportMode.Multiple;
        importer.mipmapEnabled = false;
        importer.filterMode = FilterMode.Point; // Pixel-perfect for retro sprites
        importer.textureCompression = TextureImporterCompression.Uncompressed;
        importer.alphaIsTransparency = true;
        
        // Read JSON metadata
        string json = File.ReadAllText(jsonPath);
        SpriteSheetMetadata metadata = JsonUtility.FromJson<SpriteSheetMetadata>(json);
        
        // Create sprite metadata for each frame
        List<SpriteMetaData> spritesheet = new List<SpriteMetaData>();
        
        // The extractor writes frame Y top-down; Unity's texture space is bottom-up.
        if (metadata.textureHeight <= 0 || metadata.frames == null)
        {
            Debug.LogWarning($"[StarcraftSpriteImporter] {jsonPath} has no usable " +
                             "textureHeight/frames — is it from a current extractor build?");
            return;
        }

        foreach (var frame in metadata.frames)
        {
            SpriteMetaData spriteMeta = new SpriteMetaData
            {
                name = Path.GetFileNameWithoutExtension(assetPath) + "_" + frame.index,
                rect = new Rect(frame.x, metadata.textureHeight - frame.y - frame.height, frame.width, frame.height),
                pivot = new Vector2(0.5f, 0f), // Bottom-center pivot for RTS units
                alignment = (int)SpriteAlignment.BottomCenter
            };
            spritesheet.Add(spriteMeta);
        }
        
        importer.spritesheet = spritesheet.ToArray();
    }
}

/// Mirrors the JSON written by `casc-extractor extract anim --convert-to-png`.
/// Field names must match that schema exactly: JsonUtility silently leaves unknown
/// members at their default, so a renamed field here becomes 0 rather than an error.
[System.Serializable]
public class SpriteSheetMetadata
{
    public string name;
    public int frameCount;
    /// Logical sprite bounds (the GRP-era canvas), not the atlas size.
    public int grpWidth;
    public int grpHeight;
    /// Atlas dimensions — use these to flip frame Y into Unity's bottom-up space.
    public int textureWidth;
    public int textureHeight;
    public float pixelsPerUnit;
    public FrameData[] frames;
}

[System.Serializable]
public class FrameData
{
    public int index;
    public int x;
    public int y;
    public int width;
    public int height;
}
