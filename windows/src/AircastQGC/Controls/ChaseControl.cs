using System;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Forms;
using AircastQGC.Pages.Analyze;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

public sealed partial class ChaseControl : UserControl
{
    private const string DetectionsView = "view.detections";
    private const string Instance = "vehicle.cameraManager.currentCameraInstance";
    private const string CameraSubject = "The camera";
    private const string ConfirmTitle = "Chase the target";
    private const string ConfirmText = "The aircraft will fly itself toward the locked target. Keep the area clear and be ready to take manual control.";

    private readonly string _client = $"chase.{Guid.NewGuid():N}";
    private readonly ContentControl _slot = new() { HorizontalAlignment = HorizontalAlignment.Center };
    private readonly TextBlock _reason = FormLook.Caption("", "TextSecondaryBrush");
    private (bool Engaged, bool Ready)? _shown;

    public event EventHandler<GuidedAsk>? ConfirmRequested;

    public event EventHandler<string>? Refused;

    public ChaseControl()
    {
        _reason.HorizontalAlignment = HorizontalAlignment.Center;
        _reason.TextAlignment = TextAlignment.Center;
        Content = new StackPanel { Spacing = 4, Children = { _slot, _reason } };
        Visibility = Visibility.Collapsed;
        Loaded += (_, _) =>
        {
            BridgeWatch.Watch(_client, [DetectionsView], Render);
            Render(Bridge.Get(DetectionsView));
        };
        Unloaded += (_, _) =>
        {
            if (!IsLoaded) BridgeWatch.Stop(_client);
        };
    }

    private void Render(JsonObject view)
    {
        var pursuit = PayloadJson.Pursuit(view);
        var locked = view["boxes"] is JsonArray boxes && boxes.OfType<JsonObject>().Any(box => MapUiJson.Flag(box["target"]));
        Visibility = Ui.Shown(pursuit is not null && locked);
        if (pursuit is not { } state) return;
        FormLook.Say(_reason, state.Engaged || state.Ready ? null : state.Reason);
        if (_shown == (state.Engaged, state.Ready)) return;
        _shown = (state.Engaged, state.Ready);
        _slot.Content = state.Engaged
            ? Pill("Chasing — Stop", "Stop chasing the target", ButtonLook.Stop, true, () => Ui.Off(this, StopPursuit, Refuse))
            : Pill("Chase", "Chase the locked target", ButtonLook.Accent, state.Ready, Ask);
    }

    private static Button Pill(string text, string name, ButtonLook look, bool enabled, Action onClick)
    {
        var button = FormLook.Button(text, onClick, look);
        button.MinWidth = 160;
        button.MinHeight = 40;
        button.IsEnabled = enabled;
        AutomationProperties.SetName(button, name);
        return button;
    }

    private void Ask() => ConfirmRequested?.Invoke(this, new GuidedAsk(ConfirmTitle, ConfirmText, StartPursuit));

    private static string? StartPursuit() => FormWrites.Call($"{Instance}.startPursuit", CameraSubject);

    private static string? StopPursuit() => FormWrites.Call($"{Instance}.stopPursuit", CameraSubject);

    private void Refuse(string? reason)
    {
        if (reason is not null) Refused?.Invoke(this, reason);
    }
}
