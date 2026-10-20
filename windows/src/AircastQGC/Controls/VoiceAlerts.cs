using System.Runtime.InteropServices;
using System.Text.Json.Nodes;

namespace AircastQGC.Controls;

internal sealed record ReturnAlert(bool Speak, bool Alerted)
{
    internal static ReturnAlert Next(bool alerted, bool returnNow, bool flying) =>
        new(returnNow && !alerted, flying && (alerted || returnNow));
}

internal sealed partial class VoiceAlerts(SpeechOut speech)
{
    private const string ReturnNowSpoken = "Battery needed to return home. Return now.";
    private const string ReturnNowClip = "battery_needed_to_return_home_return_now";
    private const uint LostSound = 0x10;
    private const uint RegainedSound = 0x0;

    private bool _returnNow;
    private bool _flying;
    private (bool ReturnNow, bool Flying)? _judged;
    private bool _alerted;
    private bool? _heardLost;

    internal void Battery(JsonObject view)
    {
        _returnNow = MapUiJson.Flag(view["available"]) && view["headline"] is JsonObject headline && MapUiJson.Flag(headline["returnNow"]);
        JudgeReturn();
    }

    internal void FlyState(JsonObject view)
    {
        if (MapUiJson.Text(view["class"]) != "FlyState") return;
        _flying = MapUiJson.Flag(view["flying"]);
        JudgeReturn();
        Link(MapUiJson.Flag(view["contactLost"]));
    }

    private void JudgeReturn()
    {
        if (_judged == (_returnNow, _flying)) return;
        _judged = (_returnNow, _flying);
        var next = ReturnAlert.Next(_alerted, _returnNow, _flying);
        _alerted = next.Alerted;
        if (!next.Speak) return;
        speech.Say(ReturnNowSpoken, ReturnNowClip);
        Cue(true);
    }

    private void Link(bool lost)
    {
        if (_heardLost == lost) return;
        if (_heardLost != null || lost) Cue(lost);
        _heardLost = lost;
    }

    private static void Cue(bool lost) => MessageBeep(lost ? LostSound : RegainedSound);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool MessageBeep(uint type);
}
