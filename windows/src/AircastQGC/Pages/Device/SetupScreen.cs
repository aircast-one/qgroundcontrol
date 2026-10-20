using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Pages.Device;

internal sealed record SetupNav(bool Back, bool Later, bool? Skip, bool Finish)
{
    public static SetupNav Of(int step, int total, bool held) => new(step > 1 && !held, step < total, step >= total ? null : held, step == total);
}

internal sealed class SetupScreen : DeviceScreen
{
    private int? _step;
    private bool _reconfiguring;
    private int _wizardStep = 1;

    public SetupScreen(DeviceNav nav) : base("setup", nav) { }

    public static int Clamp(int step, int total) => Math.Min(Math.Max(step, 1), Math.Max(total, 1));

    public static bool HoldsTheWizard(string key, bool shown, int wizardStep) => key == "camera" && shown && wizardStep != CameraWizardBox.Applying;

    protected override IEnumerable<Node?>? Build(DeviceReading reading, DeviceContext ctx) =>
        Viewed(reading, view => DJ.Obj(DJ.Obj(view, "camera"), "wizard") is not null && DJ.Objs(view, "steps").Count > 0, view =>
        {
            var steps = DJ.Objs(view, "steps");
            var total = steps.Count;
            var step = Clamp(_step ?? DJ.Int(view, "start", 1), total);
            var current = steps[step - 1];
            var key = DJ.Text(current, "key");
            var camera = DJ.Section(view, "camera");
            var wizard = DJ.Section(camera, "wizard");
            var wizardShown = DJ.Flag(wizard, "new") || _reconfiguring;
            var nav = SetupNav.Of(step, total, HoldsTheWizard(key, wizardShown, _wizardStep));
            var status = DJ.MaybeBadge(current, "status");
            return new Node?[]
            {
                DeviceTree.Leaf("trail", DJ.Sig(step, view["steps"]), () => Trail(steps, step)),
                DeviceTree.Leaf($"title:{key}", DJ.Sig(DJ.Text(current, "title"), status?.Text, status?.Level), () =>
                {
                    var title = FormLook.Text(DJ.Text(current, "title"), "FontSizeSubtitle", "TextPrimaryBrush", true);
                    AutomationProperties.SetHeadingLevel(title, Microsoft.UI.Xaml.Automation.Peers.AutomationHeadingLevel.Level2);
                    return FormLook.Margined<FrameworkElement>(status is null ? title : FormLook.Row(title, Kit.BadgeView(status)), new Thickness(0, 20, 0, 4));
                }),
                Kit.Text(DJ.Text(current, "lead")),
                Kit.Group($"step:{key}", key switch
                {
                    "network" => Internet(ctx, DJ.Section(view, "internet")),
                    "tailscale" => [NetworkScreen.Tailscale(ctx, DJ.Obj(view, "remote") ?? new JsonObject())],
                    "camera" => Camera(ctx, camera, wizard, wizardShown),
                    "telemetry" => Telemetry(ctx, DJ.Section(view, "telemetry")),
                    "done" => DJ.Objs(view, "checks").Select(check =>
                    {
                        var badge = DJ.Badge(check, "status");
                        return (Node?)Kit.Row(DJ.Text(check, "label"), badge.Text, "", badge.Level == DeviceLevel.Ready ? RowState.Done : RowState.Neutral,
                            () => Go(DJ.Int(check, "step", 1)), $"{DJ.Int(check, "step", 1)}");
                    }),
                    _ => [],
                }),
                Kit.Buttons("setup-nav", [
                    nav.Back ? Kit.Button("back", "← Back", () => Go(step - 1)) : null,
                    nav.Later ? Kit.Button("later", "Finish setup later", Nav.Fly, ButtonLook.Subtle) : null,
                    nav.Skip == false ? Kit.Button("next", "Next →", () => Go(step + 1), ButtonLook.Accent) : null,
                    nav.Skip == true ? Kit.Button("skip", "Skip this step →", () => Go(step + 1)) : null,
                    nav.Finish ? Kit.Action(ctx, "Go to the fly view", new DeviceRequest(DeviceLink.Post, "/api/setup/complete"), done: Nav.Fly) : null,
                ]),
            };
        });

    private void Go(int step)
    {
        _step = step;
        Rerender();
    }

    private static UIElement Trail(IReadOnlyList<JsonObject> steps, int step)
    {
        var trail = Kit.Flow(steps.SelectMany((entry, at) =>
        {
            var here = at + 1 == step;
            var done = !here && DJ.Obj(entry, "status") is not null;
            var circle = new Border
            {
                Width = 24,
                Height = 24,
                CornerRadius = new CornerRadius(12),
                Background = here ? FormLook.Brush("AccentBrush") : done ? FormLook.Brush("SuccessBrush") : null,
                BorderBrush = FormLook.Brush(here ? "AccentBrush" : "ControlStrokeBrush"),
                BorderThickness = new Thickness(done ? 0 : 1),
                Child = done
                    ? FormLook.Glyph("", 12, "BgBaseBrush")
                    : FormLook.Text($"{at + 1}", "FontSizeCaption", here ? "AccentOnBrush" : "TextSecondaryBrush"),
            };
            if (circle.Child is FrameworkElement inner)
            {
                inner.HorizontalAlignment = HorizontalAlignment.Center;
                inner.VerticalAlignment = VerticalAlignment.Center;
            }
            AutomationProperties.SetName(circle, $"Step {at + 1}{(here ? ", current" : done ? ", done" : "")}");
            return new UIElement?[]
            {
                at > 0 ? new Border { Width = 8, Height = 1, Background = FormLook.Brush("ControlStrokeBrush"), VerticalAlignment = VerticalAlignment.Center, Margin = new Thickness(0, 12, 0, 0) } : null,
                circle,
                here ? FormLook.Margined(FormLook.Caption(DJ.Text(entry, "label"), "TextPrimaryBrush"), new Thickness(0, 4, 0, 0)) : null,
            };
        }).ToArray());
        trail.Spacing = 6;
        trail.Margin = new Thickness(0, 16, 0, 0);
        AutomationProperties.SetName(trail, "Setup steps");
        return trail;
    }

    private static IEnumerable<Node?> Internet(DeviceContext ctx, JsonObject internet) =>
    [
        Kit.Notice(DJ.Badge(internet, "online")),
        NetworkScreen.WifiJoin(DJ.Section(internet, "reopen"), DJ.Flag(internet, "joinOpen")),
        Kit.PageRow(ctx, DJ.Page(DJ.Obj(internet, "connection"))),
        Kit.Text(DJ.Text(internet, "hint")),
    ];

    private IEnumerable<Node?> Camera(DeviceContext ctx, JsonObject camera, JsonObject wizard, bool shown)
    {
        var isNew = DJ.Flag(wizard, "new");
        var watch = DJ.Text(camera, "watch");
        return new Node?[] { Kit.Notice(new Badge(DeviceLevel.Warn, DJ.Text(camera, "problem"))) }.Concat(shown
            ? [DeviceTree.Kept($"wizard:{(isNew ? "" : DJ.Text(wizard, "name"))}",
                () => new CameraWizardBox(wizard, false, isNew ? null : Stop, WizardStep, _ => Stop()), box => box.Show(ctx))]
            : [
                Kit.Buttons("camera", watch.Length > 0
                    ? [Kit.Link(ctx, ctx.Url(watch), "▶ Watch live", LinkLook.Filled, true), Kit.Button("reconfigure", "Reconfigure", Reconfigure)]
                    : [Kit.Button("reconfigure", "Reconfigure", Reconfigure, ButtonLook.Accent)]),
                Kit.PageRow(ctx, DJ.Page(DJ.Obj(camera, "cameras"))),
                Kit.Text(DJ.Text(camera, "hint")),
            ]);
    }

    private void WizardStep(int step)
    {
        _wizardStep = step;
        Rerender();
    }

    private void Reconfigure()
    {
        _wizardStep = 1;
        _reconfiguring = true;
        Rerender();
    }

    private void Stop()
    {
        _reconfiguring = false;
        Rerender();
    }

    private static IEnumerable<Node?> Telemetry(DeviceContext ctx, JsonObject telemetry) =>
        new[] { Kit.Notice(DJ.MaybeBadge(telemetry, "waiting")) }
            .Concat(Kit.Endpoints(ctx, DJ.Endpoints(telemetry, "endpoints")))
            .Append(Kit.Text(DJ.Text(telemetry, "hint")));
}
