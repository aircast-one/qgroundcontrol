using System;
using System.Collections.Immutable;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading;

namespace AircastQGC.Core;

public static class VideoDriver
{
    private const string VideoView = "view.video";
    private static readonly TimeSpan Interval = TimeSpan.FromMilliseconds(500);
    private const string PreviewsPath = "video.previews";
    private const int FirstPreview = 2;
    private static readonly ImmutableArray<int> Channels = ImmutableArray.Create(VideoNative.Main, VideoNative.Pip, 2, 3, 4, 5);

    private static CancellationTokenSource? _stop;
    private static Thread? _thread;

    private sealed record Played(string? Driven, bool Restarted, string Error);

    private sealed record State(ImmutableDictionary<int, Played> Channels, string? Recording, bool RecordingReported)
    {
        internal static readonly State Idle = new(ImmutableDictionary<int, Played>.Empty, null, false);
    }

    public static void Start()
    {
        if (_thread != null || !VideoNative.Ready) return;
        Bridge.Invoke("video.setNativeRendering", new JsonArray(true));
        Bridge.Invoke("video.initNative");
        var stop = new CancellationTokenSource();
        _stop = stop;
        _thread = new Thread(() => Run(stop.Token)) { IsBackground = true, Name = "video-driver" };
        _thread.Start();
    }

    public static void Stop()
    {
        if (_thread == null) return;
        _stop?.Cancel();
        _thread.Join();
        _thread = null;
        Channels.ToList().ForEach(VideoNative.qgc_video_stop);
    }

    private static void Run(CancellationToken stop)
    {
        var state = State.Idle;
        while (!stop.WaitHandle.WaitOne(Interval)) state = Step(state);
    }

    private static State Step(State state)
    {
        var view = Bridge.Get(VideoView);
        var previews = Bridge.Get(PreviewsPath)["value"] as JsonArray;
        var wanted = Channels.ToImmutableDictionary(ch => ch, ch => ch switch
        {
            VideoNative.Main => Pipeline(view, "nativePipeline"),
            VideoNative.Pip => Pipeline(view, "pipPipeline"),
            _ => previews?.ElementAtOrDefault(ch - FirstPreview) is JsonObject preview ? Pipeline(preview, "pipeline") : null,
        });
        var stopped = Channels.Aggregate(state.Channels, (played, ch) => StopChanged(played, ch, wanted[ch]));
        var started = Channels.Aggregate(stopped, (played, ch) => StartWanted(played, ch, wanted[ch]));
        var recorded = Record(state, view["nativeRecording"] as JsonObject);
        return recorded with { Channels = started.ToImmutableDictionary(kv => kv.Key, kv => Report(kv.Key, kv.Value)) };
    }

    private static string? Pipeline(JsonObject view, string key) =>
        view[key] is JsonValue v && v.TryGetValue<string>(out var p) && p.Length > 0 ? p : null;

    private static ImmutableDictionary<int, Played> StopChanged(ImmutableDictionary<int, Played> played, int ch, string? wanted)
    {
        var restart = VideoNative.qgc_core_video_take_restart(ch);
        var driven = played.GetValueOrDefault(ch)?.Driven;
        if (driven == null || (!restart && wanted == driven)) return played;
        VideoNative.qgc_video_stop(ch);
        return played.SetItem(ch, played[ch] with { Driven = null });
    }

    private static ImmutableDictionary<int, Played> StartWanted(ImmutableDictionary<int, Played> played, int ch, string? wanted)
    {
        if (wanted == null || wanted == played.GetValueOrDefault(ch)?.Driven) return played;
        var ok = VideoNative.qgc_video_start(ch, wanted);
        return played.SetItem(ch, new Played(wanted, true, ok ? "" : VideoNative.Text(VideoNative.qgc_video_last_error(ch))));
    }

    private static State Record(State state, JsonObject? asked)
    {
        var key = asked?.ToJsonString();
        if (key != state.Recording)
        {
            if (state.Recording != null) VideoNative.qgc_video_stop_recording(VideoNative.Main);
            if (asked?["file"] is JsonValue f && f.TryGetValue<string>(out var file)
                && asked["format"] is JsonValue m && m.TryGetValue<int>(out var format))
                VideoNative.qgc_video_start_recording(VideoNative.Main, file, format);
        }
        var active = VideoNative.qgc_video_recording(VideoNative.Main);
        if (active != state.RecordingReported) Bridge.Invoke("video.reportRecording", new JsonArray(active));
        return state with { Recording = key, RecordingReported = active };
    }

    private static Played Report(int ch, Played played)
    {
        if (played.Driven == null) return played;
        var streamed = VideoNative.Text(VideoNative.qgc_video_stream_error(ch));
        Bridge.Invoke("video.reportNative", new JsonArray(
            VideoNative.qgc_video_running(ch),
            VideoNative.qgc_video_frames(ch),
            VideoNative.qgc_video_width(ch),
            VideoNative.qgc_video_height(ch),
            played.Error.Length > 0 ? played.Error : streamed,
            VideoNative.qgc_video_source_buffers(ch),
            played.Restarted,
            ch));
        return played with { Restarted = false };
    }
}
