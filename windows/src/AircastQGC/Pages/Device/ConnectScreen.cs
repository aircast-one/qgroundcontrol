using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Pages.Device;

internal sealed record GcsChoice(string Value, string Label, IReadOnlyList<string> Hides);

internal sealed class ConnectScreen : DeviceScreen
{
    private const string GcsKey = "gcs";

    private string? _saved = DeviceStore.Read(GcsKey);

    public ConnectScreen(DeviceNav nav) : base("connect", nav) { }

    protected override IEnumerable<Node?>? Build(DeviceReading reading, DeviceContext ctx) =>
        Viewed(reading, view => view["choices"] is JsonArray, view => DJ.Obj(view, "noCamera") is { } add
            ? [Kit.Section("Cameras", null, [
                Kit.Text("There is no camera on this device yet, so there is nothing to point a ground station at."),
                Kit.Buttons("no-camera", [Kit.Button("add", DJ.Text(add, "label"), () => ctx.Open(DJ.Text(add, "href")))]),
            ])]
            : Links(view, ctx));

    private IEnumerable<Node?> Links(JsonObject view, DeviceContext ctx)
    {
        var choices = DJ.Objs(view, "choices").Select(c => new GcsChoice(DJ.Text(c, "value"), DJ.Text(c, "label"), DJ.Texts(c, "hides"))).ToList();
        var choice = choices.FirstOrDefault(c => c.Value == _saved) ?? choices.FirstOrDefault();
        var telemetry = DJ.Obj(view, "telemetry");
        var trouble = DJ.Obj(view, "trouble");
        return new Node?[]
        {
            Kit.Section("Your ground station", null, [
                Kit.Buttons("gcs", choices.Select(option => (Node?)Chip(option.Value, option.Label, option.Value == choice?.Value, () => Pick(option.Value)))),
                Kit.Text("QGroundControl and Mission Planner play the RTSP link; a browser, OBS or Aircast QGC play WebRTC. This app remembers your answer."),
                Kit.Text(DJ.Text(view, "scope")),
                Kit.Disclosure("How to connect a ground station", "", null, false, () => [
                    Kit.Text("Each link below says where to paste it in your app; video and telemetry are two separate links."),
                    Kit.Text("Ground station on another device? Scan that link's QR code from there — no retyping."),
                ]),
            ]),
        }
        .Concat(DJ.Objs(view, "cameras").Select(camera => (Node?)Kit.Section(DJ.Text(camera, "name"), DJ.Badge(camera, "status"), new[]
        {
            Kit.Notice(new Badge(DeviceLevel.Warn, DJ.Text(camera, "notice"))),
            DJ.Obj(camera, "fix") is { } fix ? Kit.Buttons("fix", [Kit.Button("fix", DJ.Text(fix, "label"), () => ctx.Open(DJ.Text(fix, "href")))]) : null,
        }.Concat(Targets(ctx, camera, "targets", choice)))))
        .Append(telemetry is null ? null : Kit.Section("Telemetry", DJ.Badge(telemetry, "status"),
            Targets(ctx, telemetry, "targets", choice).Prepend(Kit.Text("Video and telemetry are separate links — a ground station needs both."))))
        .Append(trouble is null ? null : Trouble(ctx, trouble, choice));
    }

    private void Pick(string value)
    {
        DeviceStore.Write(GcsKey, value);
        _saved = value;
        Rerender();
    }

    public static Node Chip(string key, string label, bool on, Action pick) =>
        DeviceTree.Leaf($"chip:{key}", DJ.Sig(label, on), () =>
        {
            var chip = FormLook.Button(label, pick, on ? ButtonLook.Accent : ButtonLook.Standard, on ? "\uE73E" : null);
            chip.Height = 32;
            chip.MinHeight = 32;
            chip.CornerRadius = new CornerRadius(16);
            AutomationProperties.SetItemStatus(chip, on ? "Selected" : "");
            return chip;
        });

    private static Node Trouble(DeviceContext ctx, JsonObject trouble, GcsChoice? choice)
    {
        var addresses = DJ.Obj(trouble, "addresses");
        return Kit.Section("Troubleshooting", null, [
            Kit.Disclosure("This link didn't play in my app", "Reach the device by address, or fall back to a player that takes anything.", null, false, () =>
                (DJ.Objs(trouble, "fallback").Count > 0
                    ? Targets(ctx, trouble, "fallback", choice).Prepend(Kit.Sub("When nothing else will play"))
                    : [])
                .Concat(addresses is null ? [] : new Node?[]
                {
                    Kit.Sub(DJ.Text(addresses, "title")),
                    Kit.Text("Windows without Bonjour, and some networks, cannot look up a .local name — and a tailnet name needs Tailscale running there too. " +
                             "These links use this device's address instead: pick the one on the same network as your ground station."),
                }.Concat(DJ.Objs(addresses, "groups").SelectMany(group =>
                    Targets(ctx, group, "targets", choice).Prepend(Kit.Hint(DJ.Text(group, "label"))))))),
        ]);
    }

    private static IEnumerable<Node?> Targets(DeviceContext ctx, JsonObject json, string key, GcsChoice? choice) =>
        DJ.Objs(json, key)
            .Where(target => choice is null || !choice.Hides.Contains(DJ.Text(target, "proto")))
            .Select(target => (Node?)Kit.Group($"target:{DJ.Text(target, "url")}", Target(ctx, target)));

    private static IEnumerable<Node?> Target(DeviceContext ctx, JsonObject target)
    {
        var label = DJ.Text(target, "tag") is { Length: > 0 } tag ? $"{DJ.Text(target, "label")} · {tag}" : DJ.Text(target, "label");
        var alternatives = DJ.Objs(target, "alternatives");
        return new Node?[]
        {
            Kit.Copy(ctx, label, DJ.Text(target, "url"), DJ.Text(target, "hint"), DJ.Flag(target, "qr"), DJ.Flag(target, "down") ? DeviceLevel.Warn : DeviceLevel.Neutral),
            DJ.Text(target, "reach") is { Length: > 0 } reach ? DeviceTree.Kept($"reach:{reach}", () => new ReachLine(reach), _ => { }) : null,
            alternatives.Count == 0 ? null : Kit.Group("alternatives", alternatives
                .Select(alt => (Node?)Kit.Copy(ctx, DJ.Text(alt, "label"), DJ.Text(alt, "url")))
                .Prepend(Kit.Text("Doesn't resolve on Android or Windows? Use the address instead:"))),
        };
    }
}

internal sealed class ReachLine : UserControl
{
    private static readonly TimeSpan Recheck = TimeSpan.FromSeconds(30);

    private readonly string _url;
    private readonly TextBlock _said = FormLook.Caption("");
    private readonly DispatcherTimer _timer = new() { Interval = Recheck };

    public ReachLine(string url)
    {
        _url = url;
        Content = FormLook.Margined(_said, new Thickness(16, 0, 16, 8));
        _timer.Tick += (_, _) => Probe();
        Loaded += (_, _) =>
        {
            Probe();
            _timer.Start();
        };
        Unloaded += (_, _) => _timer.Stop();
    }

    private async void Probe()
    {
        if (await DeviceLink.Reach(_url) is not { } reach) return;
        _said.Text = reach ? "This address answers from here" : "No answer from here — your ground station may not reach it either";
        _said.Foreground = FormLook.Brush(reach ? "SuccessBrush" : "CautionBrush");
        _said.Visibility = Visibility.Visible;
    }
}
