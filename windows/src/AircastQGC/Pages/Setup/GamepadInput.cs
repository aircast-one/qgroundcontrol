using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading;
using AircastQGC.Core;
using Windows.Gaming.Input;

namespace AircastQGC.Pages.Setup;

internal static class GamepadInput
{
    public const string Devices = "joystick.devices";
    public const string Input = "joystick.input";
    private const int AxisCount = 6;
    private const int ButtonCount = 13;
    private const double AxisScale = 32767;
    private const int HatUp = 0x01;
    private const int HatRight = 0x02;
    private const int HatDown = 0x04;
    private const int HatLeft = 0x08;
    private static readonly TimeSpan Every = TimeSpan.FromMilliseconds(20);
    private static readonly object Gate = new();
    private static Timer? _sampler;
    private static int _sampling;

    public static void Start()
    {
        lock (Gate)
        {
            if (_sampler is not null) return;
            Gamepad.GamepadAdded += (_, _) => Report();
            Gamepad.GamepadRemoved += (_, _) => Report();
            _sampler = new Timer(_ => Sample(), null, Every, Every);
        }
        Report();
    }

    public static int Scaled(double value) => (int)Math.Round(Math.Clamp(value, -1, 1) * AxisScale);

    public static int Hat(GamepadButtons buttons) =>
        (buttons.HasFlag(GamepadButtons.DPadUp) ? HatUp : 0)
        | (buttons.HasFlag(GamepadButtons.DPadRight) ? HatRight : 0)
        | (buttons.HasFlag(GamepadButtons.DPadDown) ? HatDown : 0)
        | (buttons.HasFlag(GamepadButtons.DPadLeft) ? HatLeft : 0);

    public static IReadOnlyList<int> Axes(GamepadReading reading) =>
        new[] { reading.LeftThumbstickX, -reading.LeftThumbstickY, reading.RightThumbstickX, -reading.RightThumbstickY, reading.LeftTrigger, reading.RightTrigger }
            .Select(Scaled)
            .ToList();

    public static IReadOnlyList<bool> Buttons(GamepadReading reading) =>
    [
        reading.Buttons.HasFlag(GamepadButtons.A),
        reading.Buttons.HasFlag(GamepadButtons.B),
        reading.Buttons.HasFlag(GamepadButtons.X),
        reading.Buttons.HasFlag(GamepadButtons.Y),
        reading.Buttons.HasFlag(GamepadButtons.LeftShoulder),
        reading.Buttons.HasFlag(GamepadButtons.RightShoulder),
        reading.LeftTrigger > 0.5,
        reading.RightTrigger > 0.5,
        reading.Buttons.HasFlag(GamepadButtons.View),
        reading.Buttons.HasFlag(GamepadButtons.Menu),
        false,
        reading.Buttons.HasFlag(GamepadButtons.LeftThumbstick),
        reading.Buttons.HasFlag(GamepadButtons.RightThumbstick),
    ];

    private static IReadOnlyList<(string Name, Gamepad Pad)> Pads() =>
        Gamepad.Gamepads.Select((pad, at) => (Name(pad, at), pad)).ToList();

    private static string Name(Gamepad pad, int at) =>
        RawGameController.FromGameController(pad)?.DisplayName is { Length: > 0 } name ? name : $"Gamepad {at + 1}";

    private static void Report() =>
        Bridge.Invoke(Devices, new JsonArray(new JsonArray(Pads().Select(pad => (JsonNode?)new JsonObject
        {
            ["name"] = pad.Name,
            ["axes"] = AxisCount,
            ["buttons"] = ButtonCount,
            ["hats"] = 1,
            ["gamepad"] = true,
        }).ToArray())));

    private static void Sample()
    {
        if (Interlocked.Exchange(ref _sampling, 1) == 1) return;
        try
        {
            Pads().ToList().ForEach(pad =>
            {
                var reading = pad.Pad.GetCurrentReading();
                Bridge.Invoke(Input, new JsonArray(
                    JsonValue.Create(pad.Name),
                    new JsonArray(Axes(reading).Select(axis => (JsonNode?)JsonValue.Create(axis)).ToArray()),
                    new JsonArray(Buttons(reading).Select(button => (JsonNode?)JsonValue.Create(button)).ToArray()),
                    new JsonArray(JsonValue.Create(Hat(reading.Buttons)))));
            });
        }
        finally { Interlocked.Exchange(ref _sampling, 0); }
    }
}
