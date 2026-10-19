using System;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Input;

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

    private GuidedOffer? _pending;
    private GuidedRange? _range;
    private double? _grabbedAt;

    public event EventHandler<GuidedOffer>? Sent;
    public event EventHandler<string>? Failed;

    public GuidedOffer? Pending => _pending;
    public bool IsOpen => _pending != null;

    public GuidedConfirm()
    {
        InitializeComponent();
        Unloaded += (_, _) => BridgeWatch.Stop(WatchClient);
    }

    public bool Show(GuidedOffer offer)
    {
        if (!offer.Ready) return false;
        var range = offer.CarriesValue ? GuidedRange.From(ValueView(offer.Id, null)) : null;
        if (offer.CarriesValue && range == null)
        {
            Failed?.Invoke(this, $"{offer.Title} needs a range the vehicle has not reported.");
            return false;
        }
        _pending = offer;
        _range = range;
        Present(offer, range);
        BridgeWatch.Watch(WatchClient, new[] { GuidedOffer.ViewPath }, OnGuided);
        if (!offer.Confirm) Confirm();
        return true;
    }

    public void Confirm()
    {
        if (_pending is not GuidedOffer offer) return;
        var failure = Execute(offer, _range?.Clamped(ValueSlider.Value));
        if (failure == null)
        {
            Close();
            Sent?.Invoke(this, offer);
            return;
        }
        ErrorText.Text = failure;
        ErrorText.Visibility = Visibility.Visible;
        Failed?.Invoke(this, failure);
    }

    public void Cancel() => Close();

    public void Hide() => Close();

    private void Present(GuidedOffer offer, GuidedRange? range)
    {
        TitleText.Text = offer.Title;
        TitleText.Style = Styled(offer.Destructive ? "ConfirmTitleDestructive" : "ConfirmTitle");
        PromptText.Text = offer.Prompt;
        Knob.Style = Styled(offer.Destructive ? "KnobDestructiveStyle" : "KnobStyle");
        SlideText.Text = $"Slide to {offer.Title.ToLower(CultureInfo.CurrentCulture)}";
        OptionCheck.Content = offer.Option;
        OptionCheck.IsChecked = false;
        OptionCheck.Visibility = Seen(offer.Option.Length > 0);
        ErrorText.Visibility = Visibility.Collapsed;
        ValuePanel.Visibility = Seen(range != null);
        if (range != null) PresentRange(range);
        ReleaseKnob();
        Root.Visibility = Visibility.Visible;
    }

    private void PresentRange(GuidedRange range)
    {
        ValueLabel.Text = range.Label;
        MinimumText.Text = range.Text(range.Minimum);
        MaximumText.Text = range.Text(range.Maximum);
        ValueSlider.Minimum = Math.Min(range.Minimum, ValueSlider.Minimum);
        ValueSlider.Maximum = range.Maximum;
        ValueSlider.Minimum = range.Minimum;
        ValueSlider.Value = range.Initial;
        ValueText.Text = range.Text(range.Initial);
    }

    private void Close()
    {
        _pending = null;
        _range = null;
        ReleaseKnob();
        Root.Visibility = Visibility.Collapsed;
        BridgeWatch.Stop(WatchClient);
    }

    private void OnGuided(JsonObject view)
    {
        if (_pending is GuidedOffer pending && !GuidedOffer.List(view).Any(o => o.Id == pending.Id && o.Ready)) Close();
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
        var reached = _grabbedAt != null && Travel > 0 && KnobShift.X >= Travel - ReachSlack;
        ReleaseKnob();
        Knob.ReleasePointerCapture(e.Pointer);
        e.Handled = true;
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
        if (_range != null) ValueText.Text = _range.Text(e.NewValue);
    }

    private void CancelButton_Click(object sender, RoutedEventArgs e) => Cancel();

    private Style Styled(string key) => (Style)Resources[key];

    private static Visibility Seen(bool visible) => visible ? Visibility.Visible : Visibility.Collapsed;
}
