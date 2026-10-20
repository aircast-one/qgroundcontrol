using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.IO;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Core;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Windows.Media.Core;
using Windows.Media.Playback;
using Windows.Media.SpeechSynthesis;

namespace AircastQGC.Controls;

internal sealed record SpokenLine(long Sequence, string Text, string Clip, double Volume);

internal sealed record SpeechBatch(long Last, bool Muted, IReadOnlyList<SpokenLine> Lines)
{
    internal const string View = "view.speech";

    internal static string Path(long? after) => after is long seen ? $"{View}({seen})" : View;

    internal static SpeechBatch? Parse(JsonObject view) =>
        MapUiJson.Text(view["class"]) == "Speech"
            ? new SpeechBatch(
                (long)(MapUiJson.Number(view["last"]) ?? 0),
                MapUiJson.Flag(view["muted"]),
                MapUiJson.Objects(view["lines"])
                    .Select(line => new SpokenLine(
                        (long)(MapUiJson.Number(line["sequence"]) ?? 0),
                        MapUiJson.Text(line["text"]),
                        MapUiJson.Text(line["clip"]),
                        Math.Clamp(MapUiJson.Number(line["volume"]) ?? 1, 0, 1)))
                    .ToList())
            : null;
}

internal sealed class SpeechOut
{
    private const int MaxQueue = 20;
    private static readonly string Clips = System.IO.Path.Combine(AppContext.BaseDirectory, "Assets", "Voice");

    private readonly DispatcherQueue _ui;
    private readonly DispatcherTimer _poll = new() { Interval = TimeSpan.FromMilliseconds(500) };
    private readonly MediaPlayer _player = new() { AudioCategory = MediaPlayerAudioCategory.Speech };
    private SpeechSynthesizer? _voice;
    private long? _after;
    private bool _muted;
    private bool _polling;
    private ImmutableQueue<SpokenLine> _waiting = ImmutableQueue<SpokenLine>.Empty;
    private SpokenLine? _playing;
    private bool _playingClip;
    private int _generation;

    internal SpeechOut(DispatcherQueue ui)
    {
        _ui = ui;
        _player.CommandManager.IsEnabled = false;
        _player.MediaEnded += (_, _) => _ui.TryEnqueue(Finished);
        _player.MediaFailed += (_, _) => _ui.TryEnqueue(Failed);
        _poll.Tick += (_, _) => Poll();
    }

    internal void Start() => _poll.Start();

    internal void Stop()
    {
        _poll.Stop();
        Flush();
    }

    internal void Say(string text, string clip)
    {
        if (_after != null && !_muted) Speak([new SpokenLine(0, text, clip, 1)]);
    }

    private void Poll()
    {
        if (_polling) return;
        _polling = true;
        var asked = SpeechBatch.Path(_after);
        Task.Run(() => SpeechBatch.Parse(Bridge.Get(asked))).ContinueWith(done => _ui.TryEnqueue(() =>
        {
            _polling = false;
            if (done.IsCompletedSuccessfully) Apply(done.Result);
        }), TaskScheduler.Default);
    }

    private void Apply(SpeechBatch? batch)
    {
        if (batch == null) return;
        var seen = _after;
        _after = batch.Last;
        if (batch.Muted && !_muted) Flush();
        _muted = batch.Muted;
        if (seen is long heard) Speak(batch.Lines.Where(line => line.Sequence > heard && line.Text.Trim().Length > 0).ToList());
    }

    private void Speak(IReadOnlyList<SpokenLine> lines)
    {
        if (lines.Count == 0) return;
        if (_waiting.Count() + lines.Count > MaxQueue) Flush();
        _waiting = lines.TakeLast(MaxQueue).Aggregate(_waiting, (queue, line) => queue.Enqueue(line));
        Next();
    }

    private void Next()
    {
        if (_playing != null || _waiting.IsEmpty) return;
        _waiting = _waiting.Dequeue(out var line);
        _playing = line;
        Play(line, ClipFile(line.Clip));
    }

    private static string? ClipFile(string clip) =>
        clip.Length > 0 && System.IO.Path.Combine(Clips, clip + ".m4a") is var file && File.Exists(file) ? file : null;

    private async void Play(SpokenLine line, string? clip)
    {
        var generation = _generation;
        var source = clip != null ? MediaSource.CreateFromUri(new Uri(clip)) : await Synthesized(line.Text);
        if (generation != _generation || _playing != line) return;
        if (source == null)
        {
            Finished();
            return;
        }
        _playingClip = clip != null;
        _player.Volume = line.Volume;
        _player.Source = source;
        _player.Play();
    }

    private async Task<MediaSource?> Synthesized(string text)
    {
        try
        {
            _voice ??= new SpeechSynthesizer
            {
                Voice = SpeechSynthesizer.AllVoices.FirstOrDefault(voice => voice.Language == "en-US") ?? SpeechSynthesizer.DefaultVoice,
            };
            var stream = await _voice.SynthesizeTextToStreamAsync(text);
            return MediaSource.CreateFromStream(stream, stream.ContentType);
        }
        catch (Exception)
        {
            return null;
        }
    }

    private void Failed()
    {
        if (_playing is { } line && _playingClip) Play(line, null);
        else Finished();
    }

    private void Finished()
    {
        _playing = null;
        _playingClip = false;
        Next();
    }

    private void Flush()
    {
        _generation++;
        _waiting = ImmutableQueue<SpokenLine>.Empty;
        _playing = null;
        _playingClip = false;
        _player.Pause();
        _player.Source = null;
    }
}
