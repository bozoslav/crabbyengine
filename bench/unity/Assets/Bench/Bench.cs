// The Ponko 3D benchmark scene, rebuilt in Unity: N spinning tinted
// cubes in a field, 12 PBR spheres, a ground plane, one sun with soft
// shadows, 4x MSAA. Run the player with:
//   -cubes 100000  -seconds 10  -out result.json
// Plain Unity: one GameObject (MeshFilter, MeshRenderer, Spin script) per cube.
using System;
using System.IO;
using UnityEngine;
using UnityEngine.Rendering;
using UnityEngine.Rendering.Universal;

public static class BenchArgs
{
    public static string Get(string name, string fallback)
    {
        var args = Environment.GetCommandLineArgs();
        for (int i = 0; i < args.Length - 1; i++)
            if (args[i] == "-" + name) return args[i + 1];
        return fallback;
    }
}

/// Turns a classic GameObject cube around Y (one Update per cube).
public class Spin : MonoBehaviour
{
    public float speed;
    void Update() { transform.rotation *= Quaternion.AngleAxis(speed * Mathf.Rad2Deg * Time.deltaTime, Vector3.up); }
}

public class Bench : MonoBehaviour
{
    public static string Mode;
    public static int Cubes;
    public static Mesh CubeMesh;
    public static Material CubeMaterial;
    public static Material[] Tints;

    float seconds, warmup = 3f, t;
    string outPath;
    int frames;
    double sumFrame, sumMain, sumRender, sumGpu;
    int timed;
    FrameTiming[] timing = new FrameTiming[1];


    [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.AfterSceneLoad)]
    static void Boot() { new GameObject("Bench").AddComponent<Bench>(); }

    void Start()
    {
        Mode = "gameobjects";
        Cubes = int.Parse(BenchArgs.Get("cubes", "100000"));
        seconds = float.Parse(BenchArgs.Get("seconds", "10"), System.Globalization.CultureInfo.InvariantCulture);
        outPath = BenchArgs.Get("out", "result.json");
        QualitySettings.vSyncCount = 0;
        Application.targetFrameRate = -1;

        var urp = (UniversalRenderPipelineAsset)GraphicsSettings.currentRenderPipeline;
        urp.msaaSampleCount = 4;
        urp.shadowDistance = 60;
        urp.shadowCascadeCount = 1;
        urp.mainLightShadowmapResolution = 2048;
        urp.supportsHDR = false;

        // Camera: Ponko's orbit start (yaw 0.6, pitch 0.45, distance 18), 60° vertical.
        var cam = new GameObject("Camera").AddComponent<Camera>();
        cam.tag = "MainCamera";
        cam.fieldOfView = 60; cam.nearClipPlane = 0.1f; cam.farClipPlane = 2000;
        cam.clearFlags = CameraClearFlags.SolidColor;
        cam.backgroundColor = new Color32(0x9C, 0xC4, 0xE4, 255);
        float yaw = 0.6f, pitch = 0.45f, dist = 18;
        cam.transform.position = new Vector3(Mathf.Sin(yaw) * Mathf.Cos(pitch), Mathf.Sin(pitch), Mathf.Cos(yaw) * Mathf.Cos(pitch)) * dist;
        cam.transform.LookAt(Vector3.zero);
        var camData = cam.gameObject.AddComponent<UniversalAdditionalCameraData>();
        camData.renderPostProcessing = false;
        camData.antialiasing = AntialiasingMode.None;

        var sun = new GameObject("Sun").AddComponent<Light>();
        sun.type = LightType.Directional;
        sun.shadows = LightShadows.Soft;
        sun.intensity = 2;
        sun.transform.rotation = Quaternion.LookRotation(new Vector3(-0.8f, -0.6f, -0.4f));
        RenderSettings.ambientMode = AmbientMode.Trilight;
        RenderSettings.ambientSkyColor = (Color)new Color32(0xB8, 0xD4, 0xF0, 255) * 0.5f;
        RenderSettings.ambientGroundColor = (Color)new Color32(0x6B, 0x5B, 0x4B, 255) * 0.5f;

        // A URP Lit material shipped in Resources (Shader.Find alone gets stripped from builds).
        var lit = Resources.Load<Material>("BenchLit");
        Material Mat(Color c, float rough, float metal)
        {
            var m = new Material(lit);
            m.SetColor("_BaseColor", c);
            m.SetFloat("_Smoothness", 1 - rough);
            m.SetFloat("_Metallic", metal);
            m.enableInstancing = true;
            return m;
        }

        var ground = GameObject.CreatePrimitive(PrimitiveType.Plane);
        Destroy(ground.GetComponent<Collider>());
        ground.transform.localScale = new Vector3(40, 1, 40); // 400 x 400
        ground.GetComponent<MeshRenderer>().sharedMaterial = Mat(new Color32(0x5E, 0x8C, 0x4A, 255), 0.95f, 0);

        for (int row = 0; row < 2; row++)
            for (int col = 0; col < 6; col++)
            {
                var s = GameObject.CreatePrimitive(PrimitiveType.Sphere);
                Destroy(s.GetComponent<Collider>());
                s.transform.position = new Vector3(col * 1.1f - 2.75f, 0.45f, row * -1.2f);
                s.transform.localScale = Vector3.one * 0.9f;
                var c = row == 0 ? new Color32(0xD8, 0x45, 0x3B, 255) : new Color32(0xE8, 0xC0, 0x70, 255);
                s.GetComponent<MeshRenderer>().sharedMaterial = Mat(c, 0.05f + col * 0.19f, row);
            }

        var probe = GameObject.CreatePrimitive(PrimitiveType.Cube);
        CubeMesh = probe.GetComponent<MeshFilter>().sharedMesh;
        Destroy(probe);
        CubeMaterial = Mat(Color.white, 0.6f, 0);
        // Tints: 64 shared materials (Ponko tints per cube for free; in
        // Unity, 100k materials or property blocks would break batching).
        Tints = new Material[64];
        var tintRng = new System.Random(7);
        for (int i = 0; i < Tints.Length; i++)
            Tints[i] = Mat(new Color(0.3f + 0.7f * (float)tintRng.NextDouble(), 0.3f + 0.7f * (float)tintRng.NextDouble(), 0.3f + 0.7f * (float)tintRng.NextDouble()), 0.6f, 0);

        int side = Mathf.CeilToInt(Mathf.Sqrt(Cubes));
        var rng = new System.Random(3);
        float R(float a, float b) => a + (b - a) * (float)rng.NextDouble();
        Vector3 At(int i) => new Vector3((i % side - side / 2f) * 1.6f, 0.3f, -6f - (i / side) * 1.6f);

        {
            for (int i = 0; i < Cubes; i++)
            {
                var go = new GameObject("cube");
                go.transform.SetPositionAndRotation(At(i), Quaternion.Euler(0, R(0, 6.3f) * Mathf.Rad2Deg, 0));
                go.transform.localScale = Vector3.one * 0.6f;
                go.AddComponent<MeshFilter>().sharedMesh = CubeMesh;
                go.AddComponent<MeshRenderer>().sharedMaterial = Tints[i % Tints.Length];
                go.AddComponent<Spin>().speed = R(-2, 2);
            }
        }
        FrameTimingManager.IsFeatureEnabled();
    }

    void Update()
    {
        FrameTimingManager.CaptureFrameTimings();
        float before = t;
        t += Time.unscaledDeltaTime;
        if (t < warmup) return;
        if (before < warmup && BenchArgs.Get("shot", "") != "") ScreenCapture.CaptureScreenshot(BenchArgs.Get("shot", ""));
        frames++;
        sumFrame += Time.unscaledDeltaTime * 1000.0;
        if (FrameTimingManager.GetLatestTimings(1, timing) == 1)
        {
            timed++;
            sumMain += timing[0].cpuMainThreadFrameTime;
            sumRender += timing[0].cpuRenderThreadFrameTime;
            sumGpu += timing[0].gpuFrameTime;
        }
        if (t >= warmup + seconds)
        {
            string F(double v) => v.ToString("0.000", System.Globalization.CultureInfo.InvariantCulture);
            var json = "{\"engine\":\"unity " + Application.unityVersion + "\",\"mode\":\"" + Mode + "\",\"cubes\":" + Cubes +
                       ",\"width\":" + Screen.width + ",\"height\":" + Screen.height +
                       ",\"frames\":" + frames + ",\"fps\":" + F(frames / (sumFrame / 1000.0)) +
                       ",\"frame_ms\":" + F(sumFrame / frames) +
                       ",\"main_thread_ms\":" + F(timed > 0 ? sumMain / timed : -1) +
                       ",\"render_thread_ms\":" + F(timed > 0 ? sumRender / timed : -1) +
                       ",\"gpu_ms\":" + F(timed > 0 ? sumGpu / timed : -1) + "}";
            File.WriteAllText(outPath, json);
            Application.Quit();
        }
    }
}
