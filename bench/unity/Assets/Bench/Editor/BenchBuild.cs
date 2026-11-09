// Batch-mode build of the benchmark player:
//   Unity -batchmode -quit -projectPath . -executeMethod BenchBuild.Build
using System.Linq;
using UnityEditor;
using UnityEditor.Build;
using UnityEditor.Build.Reporting;
using UnityEditor.Rendering;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.Rendering;
using UnityEngine.Rendering.Universal;

public static class BenchBuild
{
    public static void Build()
    {
        var scene = EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
        EditorSceneManager.SaveScene(scene, "Assets/Bench/Bench.unity");

        // The PC quality level and its URP asset, with SSAO off (Ponko has none).
        var names = QualitySettings.names.ToList();
        int pc = Mathf.Max(0, names.IndexOf("PC"));
        QualitySettings.SetQualityLevel(pc, true);
        var urp = AssetDatabase.LoadAssetAtPath<UniversalRenderPipelineAsset>("Assets/Settings/PC_RPAsset.asset");
        var so = new SerializedObject(urp);
        so.FindProperty("m_SoftShadowsSupported").boolValue = true;
        so.FindProperty("m_MainLightShadowsSupported").boolValue = true;
        so.ApplyModifiedProperties();
        var renderer = AssetDatabase.LoadAssetAtPath<UniversalRendererData>("Assets/Settings/PC_Renderer.asset");
        // Remove the renderer features (SSAO): a disabled one still fails in players.
        foreach (var f in renderer.rendererFeatures.ToList()) if (f) AssetDatabase.RemoveObjectFromAsset(f);
        var rso = new SerializedObject(renderer);
        rso.FindProperty("m_RendererFeatures").ClearArray();
        rso.FindProperty("m_RendererFeatureMap").ClearArray();
        rso.ApplyModifiedProperties();
        EditorUtility.SetDirty(renderer);

        // The Lit material the player copies (keeps URP Lit in the build).
        System.IO.Directory.CreateDirectory("Assets/Bench/Resources");
        if (!AssetDatabase.LoadAssetAtPath<Material>("Assets/Bench/Resources/BenchLit.mat"))
        {
            var mat = new Material(Shader.Find("Universal Render Pipeline/Lit")) { enableInstancing = true };
            AssetDatabase.CreateAsset(mat, "Assets/Bench/Resources/BenchLit.mat");
        }
        AssetDatabase.SaveAssets();


        PlayerSettings.enableFrameTimingStats = true;
        PlayerSettings.macRetinaSupport = true;
        PlayerSettings.fullScreenMode = FullScreenMode.Windowed;
        PlayerSettings.defaultScreenWidth = 1280;
        PlayerSettings.defaultScreenHeight = 720;
        PlayerSettings.resizableWindow = false;
        PlayerSettings.runInBackground = true;
        PlayerSettings.SetScriptingBackend(NamedBuildTarget.Standalone, ScriptingImplementation.Mono2x);
        UnityEditor.OSXStandalone.UserBuildSettings.architecture = UnityEditor.Build.OSArchitecture.ARM64;

        var report = BuildPipeline.BuildPlayer(new BuildPlayerOptions
        {
            scenes = new[] { "Assets/Bench/Bench.unity" },
            locationPathName = "Build/CrabbyBench3D.app",
            target = BuildTarget.StandaloneOSX,
            options = BuildOptions.None,
        });
        Debug.Log("BENCH BUILD " + report.summary.result + " " + report.summary.totalSize);
        if (report.summary.result != BuildResult.Succeeded) EditorApplication.Exit(1);
    }
}
