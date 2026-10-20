using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading;
using System.Threading.Tasks;
using AircastQGC.Core;
using AircastQGC.Pages.Settings;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Pages.Device;

internal sealed record DeviceReading(string Origin, string Host, bool Answered, JsonObject? View)
{
    public static DeviceReading Of(JsonObject? json) =>
        new(DJ.Text(json, "origin"), DJ.Text(json, "host"), DJ.Flag(json, "answered"), DJ.Obj(json, "view"));
}

internal sealed record DeviceNav(Action<string> Open, Action Close, Action Fly, string Href)
{
    public IReadOnlyDictionary<string, string> Query => DeviceLink.Split(Href).Query;

    public string Fragment => DeviceLink.Fragment(Href);
}

internal abstract class DeviceScreen : UserControl
{
    private static int _next;
    private static readonly TimeSpan ToastLife = TimeSpan.FromSeconds(4);

    private readonly string _path;
    private readonly string _client;
    private readonly StackPanel _body = new();
    private readonly InfoBar _toast = new()
    {
        IsOpen = false,
        IsClosable = true,
        Severity = InfoBarSeverity.Informational,
        HorizontalAlignment = HorizontalAlignment.Center,
        VerticalAlignment = VerticalAlignment.Bottom,
        Margin = new Thickness(16),
        MaxWidth = 480,
    };
    private readonly DispatcherTimer _toastTimer = new() { Interval = ToastLife };
    private DeviceReading? _last;

    protected DeviceScreen(string page, DeviceNav nav)
    {
        Nav = nav;
        _path = DeviceLink.ViewPath(page);
        _client = $"device.{page}.{Interlocked.Increment(ref _next)}";
        _toastTimer.Tick += (_, _) =>
        {
            _toastTimer.Stop();
            _toast.IsOpen = false;
        };
        Content = new Grid { Children = { SettingsBodies.Scrolled(_body), _toast } };
        Loaded += async (_, _) =>
        {
            BridgeWatch.Watch(_client, [_path], Apply);
            Apply(await Task.Run(() => Bridge.Get(_path)));
        };
        Unloaded += (_, _) => BridgeWatch.Stop(_client);
    }

    protected DeviceNav Nav { get; }

    public void Say(string text)
    {
        _toast.Message = text;
        _toast.IsOpen = true;
        _toastTimer.Stop();
        _toastTimer.Start();
    }

    protected void Rerender()
    {
        if (_last is { } reading) Paint(reading);
    }

    protected abstract IEnumerable<Node?>? Build(DeviceReading reading, DeviceContext ctx);

    protected static IEnumerable<Node?>? Viewed(DeviceReading reading, Func<JsonObject, bool> parses, Func<JsonObject, IEnumerable<Node?>> build) =>
        reading.View is { } view && parses(view) ? build(view) : null;

    private void Apply(JsonObject json)
    {
        _last = DeviceReading.Of(json);
        Paint(_last);
    }

    private void Paint(DeviceReading reading)
    {
        var ctx = new DeviceContext(reading.Origin, Nav.Open, Nav.Close, Say);
        DeviceTree.Sync(_body, (Build(reading, ctx) ?? [Kit.Hint(Kit.Waiting)]).OfType<Node>());
    }
}

internal static class DeviceScreens
{
    private static readonly IReadOnlyDictionary<string, (string Title, Func<DeviceNav, UIElement> Make)> All =
        new Dictionary<string, (string, Func<DeviceNav, UIElement>)>
        {
            ["/telemetry"] = ("Telemetry", nav => new TelemetryScreen(nav)),
            ["/connect"] = ("Ground station links", nav => new ConnectScreen(nav)),
            ["/flights"] = ("Flight log", nav => new FlightsScreen(nav)),
            ["/settings/device"] = ("Device", nav => new DeviceSettingsScreen(nav)),
            ["/settings/engineering"] = ("Engineering", nav => new EngineeringScreen(nav)),
            ["/settings/video"] = ("Video engine", nav => new VideoEngineScreen(nav)),
            ["/settings/recordings"] = ("Recordings", nav => new RecordingsScreen(nav)),
            ["/network"] = ("Connection", nav => new NetworkScreen(nav)),
            ["/streaming"] = ("Cameras", nav => new CamerasScreen(nav)),
            ["/settings/camera"] = ("Watching & recording", nav => new CameraSetupScreen(nav)),
            ["/setup"] = ("Device setup", nav => new SetupScreen(nav)),
        };

    public static bool Native(string href) => All.ContainsKey(DeviceLink.Split(href).Path);

    public static UIElement Home(SettingsNav settings) => new DeviceHome(Navigator(settings, ""));

    private static DeviceNav Navigator(SettingsNav settings, string href) =>
        new(next => Open(settings, next), settings.Pop, Controls.FlyRequests.OpenFly, href);

    private static void Open(SettingsNav settings, string href)
    {
        if (All.TryGetValue(DeviceLink.Split(href).Path, out var entry)) settings.Push(entry.Title, entry.Make(Navigator(settings, href)));
    }
}
