using System.IO;
using System.Linq;
using UnityEditor;
using UnityEditor.Animations;
using UnityEngine;

/// <summary>
/// Creates Unity Animation Clips from StarCraft sprite sheets.
/// Usage: Select sprite sheets in Project window, then Tools > StarCraft > Create Animations
/// </summary>
public class StarcraftAnimationCreator : EditorWindow
{
    private int framesPerSecond = 15; // StarCraft runs at ~15 FPS for animations
    
    [MenuItem("Tools/StarCraft/Create Animations from Selected Sprites")]
    static void CreateAnimations()
    {
        var window = GetWindow<StarcraftAnimationCreator>("SC Animation Creator");
        window.Show();
    }
    
    void OnGUI()
    {
        GUILayout.Label("StarCraft Animation Creator", EditorStyles.boldLabel);
        GUILayout.Space(10);
        
        framesPerSecond = EditorGUILayout.IntSlider("Frames Per Second", framesPerSecond, 1, 60);
        
        GUILayout.Space(10);
        
        if (GUILayout.Button("Create Animations from Selected Sprites", GUILayout.Height(40)))
        {
            CreateAnimationsFromSelection();
        }
        
        GUILayout.Space(10);
        EditorGUILayout.HelpBox(
            "1. Select sprite sheets in Project window\n" +
            "2. Click 'Create Animations'\n" +
            "3. Animations will be created in the same folder",
            MessageType.Info);
    }
    
    void CreateAnimationsFromSelection()
    {
        var textures = Selection.GetFiltered<Texture2D>(SelectionMode.Assets);
        
        if (textures.Length == 0)
        {
            EditorUtility.DisplayDialog("No Sprites Selected", 
                "Please select sprite sheets in the Project window.", "OK");
            return;
        }
        
        int created = 0;
        foreach (var texture in textures)
        {
            if (CreateAnimationForSprite(texture))
                created++;
        }
        
        AssetDatabase.SaveAssets();
        AssetDatabase.Refresh();
        
        EditorUtility.DisplayDialog("Animations Created", 
            $"Created {created} animation clips.", "OK");
    }
    
    bool CreateAnimationForSprite(Texture2D texture)
    {
        string path = AssetDatabase.GetAssetPath(texture);
        string directory = Path.GetDirectoryName(path);
        string filename = Path.GetFileNameWithoutExtension(path);
        
        // Load all sprites from the texture
        Object[] sprites = AssetDatabase.LoadAllAssetsAtPath(path)
            .Where(obj => obj is Sprite)
            .ToArray();
        
        if (sprites.Length == 0)
        {
            Debug.LogWarning($"No sprites found in {filename}");
            return false;
        }
        
        // Create animation clip
        AnimationClip clip = new AnimationClip();
        clip.frameRate = framesPerSecond;
        
        // Create sprite animation
        EditorCurveBinding spriteBinding = new EditorCurveBinding();
        spriteBinding.type = typeof(SpriteRenderer);
        spriteBinding.path = "";
        spriteBinding.propertyName = "m_Sprite";
        
        ObjectReferenceKeyframe[] keyframes = new ObjectReferenceKeyframe[sprites.Length];
        for (int i = 0; i < sprites.Length; i++)
        {
            keyframes[i] = new ObjectReferenceKeyframe
            {
                time = i / (float)framesPerSecond,
                value = sprites[i]
            };
        }
        
        AnimationUtility.SetObjectReferenceCurve(clip, spriteBinding, keyframes);
        
        // Set loop settings
        AnimationClipSettings settings = AnimationUtility.GetAnimationClipSettings(clip);
        settings.loopTime = true;
        AnimationUtility.SetAnimationClipSettings(clip, settings);
        
        // Save animation clip
        string animPath = Path.Combine(directory, filename + "_anim.anim");
        AssetDatabase.CreateAsset(clip, animPath);
        
        Debug.Log($"Created animation: {animPath}");
        return true;
    }
}
