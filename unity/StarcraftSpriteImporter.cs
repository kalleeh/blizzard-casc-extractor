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
        
        foreach (var frame in metadata.frames)
        {
            SpriteMetaData spriteMeta = new SpriteMetaData
            {
                name = Path.GetFileNameWithoutExtension(assetPath) + "_" + frame.index,
                rect = new Rect(frame.x, metadata.sheetHeight - frame.y - frame.height, frame.width, frame.height),
                pivot = new Vector2(0.5f, 0f), // Bottom-center pivot for RTS units
                alignment = (int)SpriteAlignment.BottomCenter
            };
            spritesheet.Add(spriteMeta);
        }
        
        importer.spritesheet = spritesheet.ToArray();
    }
}

[System.Serializable]
public class SpriteSheetMetadata
{
    public int frameCount;
    public int frameWidth;
    public int frameHeight;
    public int framesPerRow;
    public int rows;
    public int sheetWidth;
    public int sheetHeight;
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
