using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media.Animation;
using Windows.System;

namespace AircastQGC.Controls;

public sealed partial class GuidedConfirm : UserControl
{
    private const string WatchClient = "guidedConfirm";
    private const double ClimbOutAltitude = 50.0;
    private const int GripperRelease = 0;
    private const int GripperGrab = 1;
    private const int GripperHold = 2;
    private const double ReachSlack = 0.5;
    private const string Unanswered = "The vehicle has not reported what this action would send.";
    private const string Refused = "The vehicle was not sent the command.";
    private static readonly TimeSpan ProbeSettle = TimeSpan.FromMilliseconds(150);
    private static readonly TimeSpan HoldToConfirm = TimeSpan.FromMilliseconds(800);
    private static readonly TimeSpan HoldToConfirmDestructive = TimeSpan.FromMilliseconds(1500);
    private static readonly IReadOnlySet<string> Probed = new HashSet<string> { "changeAltitude", "pause", "changeSpeed" };
    private static readonly IReadOnlySet<string> SentenceOnly = new HashSet<string> { "changeAltitude", "pause" };

    private readonly DispatcherTimer _settle = new() { Interval = ProbeSettle };
    private readonly Storyboard _hold = new();
    private readonly DoubleAnimation _holdFill = new() { From = 0, To = 1 };

    private GuidedAsk? _ask;
    private GuidedOffer? _pending;
    private GuidedRange? _range;
    private double? _grabbedAt;
    private IReadOnlyList<(GuidedPick Pick, Action<bool> Mark, UIElement View)> _picks = [];
    private bool _pointered;

    public event EventHandler<GuidedOffer>? Sent;
    public event EventHandler<string>? Failed;

    public GuidedOffer? Pending => _pending;
    public bool IsOpen => _ask != null;

    public GuidedConfirm()
    {
        InitializeComponent();
        _settle.Tick += (_, _) => Probe();
        Storyboard.SetTarget(_holdFill, HoldScale);
        Storyboard.SetTargetProperty(_holdFill, "ScaleX");
        _hold.Children.Add(_holdFill);
        _hold.Completed += (_, _) => Confirm();
        ConfirmButton.AddHandler(PointerPressedEvent, new PointerEventHandler((_, _) => Hold()), true);
        ConfirmButton.AddHandler(PointerReleasedEvent, new PointerEventHandler((_, _) => LetGo()), true);
        ConfirmButton.AddHandler(PointerCanceledEvent, new PointerEventHandler((_, _) => LetGo()), true);
        ConfirmButton.AddHandler(PointerCaptureLostEvent, new PointerEventHandler((_, _) => LetGo()), true);
        ConfirmButton.AddHandler(PointerExitedEvent, new PointerEventHandler((_, _) => LetGo()), true);
        Unloaded += (_, _) =>
        {
            BridgeWatch.Stop(WatchClient);
            _settle.Stop();
        };
    }

    public bool Show(GuidedOffer offer, bool focus = true)
    {
        if (!offer.Ready) return false;
        var range = offer.CarriesValue ? GuidedRange.From(ValueView(offer.Id, null)) : null;
        if (offer.CarriesValue && range == null)
        {
            Failed?.Invoke(this, $"{offer.Title} needs a range the vehicle has not reported.");
            return false;
        }
        Open(new GuidedAsk(offer.Title, offer.Prompt, () => Execute(offer, _range?.Clamped(ValueSlider.Value)))
        {
            Destructive = offer.Destructive,
            Fields = GuidedExtras.For(offer.Id),
            WatchPath = GuidedOffer.ViewPath,
            StillOffered = view => GuidedOffer.List(view).Any(o => o.Id == offer.Id && o.Ready),
        }, offer, range, focus);
        if (!offer.Confirm) Confirm();
        return true;
    }

    public void Ask(GuidedAsk ask) => Open(ask, null, null, true);

    public void Confirm()
    {
        if (_ask is not GuidedAsk ask) return;
        var offer = _pending;
        var failure = ask.Send();
        if (failure != null)
        {
            Refuse(failure);
            return;
        }
        Close();
        if (offer != null) Sent?.Invoke(this, offer);
    }

    public void Cancel() => Close();

    public void Hide() => Close();

    private void Open(GuidedAsk ask, GuidedOffer? offer, GuidedRange? range, bool focus)
    {
        _ask = ask;
        _pending = offer;
        _range = range;
        TitleText.Text = ask.Title;
        TitleText.Style = Styled(ask.Destructive ? "ConfirmTitleDestructive" : "ConfirmTitle");
        PromptText.Text = SentenceOnly.Contains(offer?.Id ?? "") && range != null ? "" : ask.Prompt;
        PromptText.Visibility = Seen(PromptText.Text.Length > 0);
        WarningText.Text = ask.Warning;
        WarningText.Visibility = Seen(ask.Warning.Length > 0);
        FieldsHost.Content = ask.Fields;
        FieldsHost.Visibility = Seen(ask.Fields != null);
        DetailText.Text = ask.Detail;
        DetailText.Visibility = Seen(ask.Detail.Length > 0);
        AlternativeButton.Content = ask.Alternative?.Title;
        AlternativeButton.Visibility = Seen(ask.Alternative != null);
        Knob.Style = Styled(ask.Destructive ? "KnobDestructiveStyle" : "KnobStyle");
        SlideText.Text = $"Slide to {Phrase(ask.Title)}";
        ConfirmButton.Style = Styled(ask.Destructive ? "HoldButtonDestructive" : "HoldButton");
        HoldFill.Background = ConfirmButton.Foreground;
        ConfirmText.Text = $"Confirm {Phrase(ask.Title)}";
        AutomationProperties.SetName(ConfirmButton, ConfirmText.Text);
        ToolTipService.SetToolTip(ConfirmButton, $"{ConfirmText.Text}. Hold to confirm, or press Enter.");
        _holdFill.Duration = ask.Destructive ? HoldToConfirmDestructive : HoldToConfirm;
        OptionCheck.Content = offer?.Option ?? "";
        OptionCheck.IsChecked = false;
        OptionCheck.Visibility = Seen(offer?.Option.Length > 0);
        ErrorText.Visibility = Visibility.Collapsed;
        ValuePanel.Visibility = Seen(range != null);
        EndTyping();
        Sendable(!Probed.Contains(offer?.Id ?? "") || range == null);
        if (range != null) PresentRange(range);
        if (range != null) Probe();
        ReleaseKnob();
        ReleaseHold();
        Root.Visibility = Visibility.Visible;
        BridgeWatch.Watch(WatchClient, ask.WatchPath.Length > 0 ? new[] { ask.WatchPath } : Array.Empty<string>(), OnWatched);
        if (focus) DispatcherQueue.TryEnqueue(() => (ask.Destructive ? CancelButton : ConfirmButton).Focus(FocusState.Programmatic));
    }

    private static string Phrase(string title) =>
        string.Join(' ', title.Split(' ').Select(word => word.Length > 1 && word.All(char.IsUpper) ? word : word.ToLower(CultureInfo.CurrentCulture)));

    private void PresentRange(GuidedRange range)
    {
        ValueLabel.Text = range.Label;
        MinimumText.Text = range.Text(range.Minimum);
        MaximumText.Text = range.Text(range.Maximum);
        ValueSlider.Minimum = Math.Min(range.Minimum, ValueSlider.Minimum);
        ValueSlider.Maximum = range.Maximum;
        ValueSlider.Minimum = range.Minimum;
        ValueSlider.Value = range.Initial;
        ValueText.Text = GuidedPicks.Reading(range.Initial, range.Unit);
        BuildPicks(range);
    }

    private void BuildPicks(GuidedRange range)
    {
        _picks = GuidedPicks.For(_pending?.Id ?? "", range).Select(pick => Pick(pick, range)).ToList();
        FormLook.Fill(PicksRow, _picks.Select(pick => pick.View));
        PicksRow.Visibility = Seen(_picks.Count > 0);
        PaintPicks();
    }

    private (GuidedPick, Action<bool>, UIElement) Pick(GuidedPick pick, GuidedRange range)
    {
        void Choose()
        {
            ValueSlider.Value = range.Clamped(pick.Target(ValueSlider.Value));
            PaintPicks();
        }
        if (!pick.Preset)
        {
            var step = FormLook.Button(pick.Label, Choose);
            return (pick, same => step.IsEnabled = !same, step);
        }
        var chip = new RcControlsLayer.Chip(pick.Label, _ => Choose());
        return (pick, chip.Set, chip.View);
    }

    private void PaintPicks()
    {
        if (_range is not { } range) return;
        var value = ValueSlider.Value;
        _picks.ToList().ForEach(pick => pick.Mark(GuidedPicks.Same(pick.Pick.Target(value), value, range.Unit)));
    }

    private void Close()
    {
        _ask = null;
        _pending = null;
        _range = null;
        _picks = [];
        _settle.Stop();
        EndTyping();
        PicksRow.Children.Clear();
        FieldsHost.Content = null;
        ReleaseKnob();
        ReleaseHold();
        Root.Visibility = Visibility.Collapsed;
        BridgeWatch.Stop(WatchClient);
    }

    private void Refuse(string failure)
    {
        ErrorText.Text = failure;
        ErrorText.Visibility = Visibility.Visible;
        Failed?.Invoke(this, failure);
    }

    private void OnWatched(JsonObject view)
    {
        if (_ask is GuidedAsk ask && !ask.StillOffered(view)) Close();
    }

    private string? Execute(GuidedOffer offer, double? chosen) => offer.Id switch
    {
        "arm" => Write("vehicle.armed", true, "the vehicle to armed"),
        "disarm" => Write("vehicle.armed", false, "the vehicle to disarmed"),
        "rtl" => Call("vehicle.guidedModeRTL", Args(OptionCheck.IsChecked == true)),
        "land" => Call("vehicle.guidedModeLand"),
        "takeoff" => Takeoff(chosen),
        "changeAltitude" => ChangeAltitude(ValueView(offer.Id, chosen), false),
        "pause" => ChangeAltitude(ValueView(offer.Id, chosen), true),
        "changeSpeed" => ChangeSpeed(ValueView(offer.Id, chosen)),
        "startMission" or "continueMission" => Call("vehicle.startMission"),
        "resumeMission" => ResumeMission(),
        "cancelRoi" => Call("vehicle.stopGuidedModeROI"),
        "landAbort" => Call("vehicle.abortLanding", Args(ClimbOutAltitude)),
        "emergencyStop" => Call("vehicle.emergencyStop"),
        "grab" => Gripper(GripperGrab),
        "release" => Gripper(GripperRelease),
        "hold" => Gripper(GripperHold),
        "vtolTransitionToFixedWing" => Write("vehicle.vtolInFwdFlight", true, "the VTOL to fixed-wing flight"),
        "vtolTransitionToMultiRotor" => Write("vehicle.vtolInFwdFlight", false, "the VTOL to multi-rotor flight"),
        "forceArm" => Call("vehicle.forceArm"),
        _ => $"{offer.Title} is not available in this build.",
    };

    private static string? Takeoff(double? chosen) =>
        chosen == null
            ? Call("vehicle.guidedModeTakeoff")
            : GuidedJson.Number(ValueView("takeoff", chosen), "targetMeters") is double metres
                ? Call("vehicle.guidedModeTakeoff", Args(metres))
                : Unanswered;

    private static string? ChangeAltitude(JsonObject answer, bool pause) =>
        GuidedJson.Number(answer, "deltaMeters") is not double delta
            ? Unanswered
            : !GuidedJson.Flag(answer, "sends", false)
                ? GuidedJson.FirstText(GuidedJson.Text(answer, "sentence"), Unanswered)
                : Call("vehicle.guidedModeChangeAltitude", Args(delta, pause));

    private static string? ChangeSpeed(JsonObject answer) =>
        GuidedJson.Text(answer, "command") is { Length: > 0 } command
        && GuidedJson.Number(answer, "targetMetersSecond") is double speed
            ? Call($"vehicle.{command}", Args(speed))
            : Unanswered;

    private static string? ResumeMission() =>
        GuidedJson.Whole(Bridge.Get(GuidedOffer.ViewPath), "resumeFromSequence") is int sequence && sequence > 0
            ? Call("planFly.missionController.resumeMission", Args(sequence))
            : "There is no waypoint to resume from.";

    private static string? Gripper(int action) =>
        Call("vehicle.sendGripperAction", Args(action, NextBay()));

    private static int NextBay() =>
        GuidedJson.Whole(Bridge.Get(GuidedOffer.ViewPath)["payload"] as JsonObject, "next") ?? 0;

    private static JsonObject ValueView(string id, double? target)
    {
        var argument = target?.ToString(CultureInfo.InvariantCulture);
        return id switch
        {
            "takeoff" => Bridge.Get(ViewCall("view.guidedTakeoff", argument)),
            "changeAltitude" => Bridge.Get(ViewCall("view.guidedAltitude", argument)),
            "pause" => Bridge.Get(ViewCall("view.guidedAltitude", argument == null ? null : $"{argument},pause")),
            "changeSpeed" => Bridge.Get(ViewCall("view.guidedSpeed", argument)),
            _ => new JsonObject(),
        };
    }

    private static string ViewCall(string path, string? argument) =>
        argument == null ? path : $"{path}({argument})";

    private static JsonArray Args(params JsonNode?[] items) => new(items);

    private static string? Call(string path, JsonArray? args = null) =>
        Refusal(Bridge.Invoke(path, args), Refused);

    private static string? Write(string path, bool value, string what) =>
        Refusal(Bridge.Set(path, value), $"Could not change {what}. It is unchanged.");

    private static string? Refusal(JsonObject answer, string fallback) =>
        GuidedJson.Flag(answer, "ok", false)
            ? null
            : GuidedJson.FirstText(GuidedJson.Text(answer, "reason"), GuidedJson.Text(answer, "error"), fallback);

    private double Travel => Math.Max(0, SlideTrack.ActualWidth - Knob.ActualWidth);

    private void Knob_PointerPressed(object sender, PointerRoutedEventArgs e)
    {
        _grabbedAt = Knob.CapturePointer(e.Pointer) ? e.GetCurrentPoint(SlideTrack).Position.X : null;
        e.Handled = true;
    }

    private void Knob_PointerMoved(object sender, PointerRoutedEventArgs e)
    {
        if (_grabbedAt is double from) KnobShift.X = Math.Clamp(e.GetCurrentPoint(SlideTrack).Position.X - from, 0, Travel);
    }

    private void Knob_PointerReleased(object sender, PointerRoutedEventArgs e)
    {
        e.Handled = true;
        Slid();
        Knob.ReleasePointerCapture(e.Pointer);
    }

    private void Slid()
    {
        var reached = _grabbedAt != null && Travel > 0 && KnobShift.X >= Travel - ReachSlack;
        ReleaseKnob();
        if (reached) Confirm();
    }

    private void Knob_PointerEnded(object sender, PointerRoutedEventArgs e) => ReleaseKnob();

    private void ReleaseKnob()
    {
        _grabbedAt = null;
        KnobShift.X = 0;
    }

    private void ValueSlider_ValueChanged(object sender, RangeBaseValueChangedEventArgs e)
    {
        if (_range == null) return;
        ValueText.Text = GuidedPicks.Reading(e.NewValue, _range.Unit);
        PaintPicks();
        if (Probed.Contains(_pending?.Id ?? "")) _settle.Start();
    }

    private async void Probe()
    {
        _settle.Stop();
        if (_pending is not { } offer || _ask is not { } ask || _range == null || !Probed.Contains(offer.Id)) return;
        var at = ValueSlider.Value;
        var view = await Task.Run(() => ValueView(offer.Id, at));
        if (_pending != offer || ValueSlider.Value != at) return;
        PromptText.Text = string.Join("\n", new[] { SentenceOnly.Contains(offer.Id) ? "" : ask.Prompt, GuidedJson.Text(view, "sentence") }.Where(part => part.Length > 0));
        PromptText.Visibility = Seen(PromptText.Text.Length > 0);
        Sendable(offer.Id == "changeSpeed" ? GuidedJson.Text(view, "command").Length > 0 : GuidedJson.Flag(view, "sends", false));
    }

    private void Sendable(bool sendable)
    {
        Knob.IsHitTestVisible = sendable;
        SlideTrack.Opacity = sendable ? 1 : 0.4;
        ConfirmButton.IsEnabled = sendable;
        if (!sendable) ReleaseHold();
    }

    private void Hold()
    {
        _pointered = true;
        if (ConfirmButton.IsEnabled && _ask != null) _hold.Begin();
    }

    private void LetGo()
    {
        ReleaseHold();
        DispatcherQueue.TryEnqueue(Microsoft.UI.Dispatching.DispatcherQueuePriority.Low, () => _pointered = false);
    }

    private void ReleaseHold() => _hold.Stop();

    private void ConfirmButton_Click(object sender, RoutedEventArgs e)
    {
        if (_pointered) _pointered = false;
        else Confirm();
    }

    private void ConfirmButton_PreviewKeyDown(object sender, KeyRoutedEventArgs e) => e.Handled = Key(e.Key, e.KeyStatus.WasKeyDown);

    private void ConfirmButton_PreviewKeyUp(object sender, KeyRoutedEventArgs e) => e.Handled = e.Key is VirtualKey.Enter or VirtualKey.Space;

    private bool Key(VirtualKey key, bool repeat)
    {
        if (key == VirtualKey.Escape) Cancel();
        else if (key is VirtualKey.Enter or VirtualKey.Space && !repeat && ConfirmButton.IsEnabled) Confirm();
        return key is VirtualKey.Escape or VirtualKey.Enter or VirtualKey.Space;
    }

    private void Step(int delta)
    {
        if (_range is { } range) ValueSlider.Value = GuidedPicks.Stepped(ValueSlider.Value, delta, range);
    }

    private void StepDown_Click(object sender, RoutedEventArgs e) => Step(-1);

    private void StepUp_Click(object sender, RoutedEventArgs e) => Step(1);

    private void ValueButton_Click(object sender, RoutedEventArgs e)
    {
        if (_range is not { } range) return;
        ValueEntry.Text = GuidedPicks.Number(ValueSlider.Value, range.Unit);
        ValueButton.Visibility = Visibility.Collapsed;
        ValueEntry.Visibility = Visibility.Visible;
        ValueEntry.Focus(FocusState.Programmatic);
        ValueEntry.SelectAll();
    }

    private void ValueEntry_TextChanged(object sender, TextChangedEventArgs e)
    {
        if (ValueEntry.Visibility == Visibility.Visible && _range is { } range && GuidedPicks.Typed(ValueEntry.Text, range) is double typed)
            ValueSlider.Value = typed;
    }

    private void ValueEntry_KeyDown(object sender, KeyRoutedEventArgs e)
    {
        if (e.Key != Windows.System.VirtualKey.Enter) return;
        e.Handled = true;
        EndTyping();
    }

    private void ValueEntry_LostFocus(object sender, RoutedEventArgs e) => EndTyping();

    private void EndTyping()
    {
        ValueEntry.Visibility = Visibility.Collapsed;
        ValueButton.Visibility = Visibility.Visible;
    }

    private void CancelButton_Click(object sender, RoutedEventArgs e) => Cancel();

    private void AlternativeButton_Click(object sender, RoutedEventArgs e)
    {
        if (_ask?.Alternative is not { } alternative) return;
        if (alternative.Send() is string failure) Refuse(failure);
        else Close();
    }

    private Style Styled(string key) => (Style)Resources[key];

    private static Visibility Seen(bool visible) => visible ? Visibility.Visible : Visibility.Collapsed;
}
