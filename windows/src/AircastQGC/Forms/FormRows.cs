using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;
using AircastQGC.Controls;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Windows.System;

namespace AircastQGC.Forms;

internal sealed partial class FormRows(FormView host)
{
    public const string DangerNote =
        "If this happens in flight, the motors stop and the aircraft falls. Only choose it if a falling aircraft is safer than a flying one.";

    [GeneratedRegex("(?i)(terminat|disarm|stop motors|kill)")]
    private static partial Regex DangerousChoice();

    public static bool Dangerous(string label) => DangerousChoice().IsMatch(label);

    public UIElement Row(FormControl c, FormBlock block) => FormLook.Stack(0,
        Indented(Body(c, block), c.Indent),
        c.Calculator is { } calculator && host.HasCalculator ? CalculatorRow(c, calculator) : null);

    public static UIElement Labels(IEnumerable<FormControl> run) =>
        FormLook.Fill(new StackPanel { Padding = new Thickness(16, 12, 16, 12), Spacing = 4 }, run.Select(c => Indented(LabelText(c), c.Indent)));

    private static FrameworkElement Indented(FrameworkElement body, bool indent) =>
        indent ? FormLook.Margined(body, new Thickness(body.Margin.Left + 24, body.Margin.Top, body.Margin.Right, body.Margin.Bottom)) : body;

    private FrameworkElement Body(FormControl c, FormBlock block) => c switch
    {
        { IsLabel: true } => LabelRow(c),
        { IsButton: true } => ActionRow(c, false),
        { IsDialog: true } => ActionRow(c, true),
        { Slider: not null, IsBitmask: false } => SliderRow(c, block),
        { Understood: false } => ValueRow(c, block, "Not editable here"),
        { IsToggle: true } => ToggleRow(c, block),
        { ReadOnly: true } => ValueRow(c, block, ""),
        { IsChoice: true } => ChoiceRow(c, block),
        { IsBitmask: true } => BitsRow(c, block),
        { Name: AspectRatioText.Setting } => TextRow(c, block, AspectRatioText.Shown(c), AspectRatioText.Stored),
        { IsText: true } => TextRow(c, block),
        { Number: null, ValueString.Length: > 0 } => TextRow(c, block),
        _ => NumberRow(c, block),
    };

    private (StackPanel Left, TextBlock Refusal, TextBlock Hint) Describe(FormControl c, FormBlock block, string extra = "")
    {
        var refusal = FormLook.Caption("", "CriticalBrush");
        var hint = FormLook.Caption("", "TextTertiaryBrush");
        var left = FormLook.Stack(2,
            FormLook.Text(MapUiJson.SentenceCase(c.Heading), "FontSizeBody", host.Highlighted.Contains(c.Name) ? "CautionBrush" : "TextPrimaryBrush"),
            FormLook.Caption(FormText.Joined(c.Detail, extra)),
            FormLook.Caption(!c.AcceptsWrite && c.InertNote != block.SharedInertNote ? c.InertNote : "", "TextTertiaryBrush"),
            FormLook.Caption(c.Problem, "TextTertiaryBrush"),
            FormLook.Caption(c.RebootNote != block.SharedRebootNote ? c.RebootNote : "", "CautionBrush"),
            hint,
            refusal);
        if (c.LongDescription.Length > 0) ToolTipService.SetToolTip(left, c.LongDescription);
        return (left, refusal, hint);
    }

    private async void Commit(FormControl c, TextBlock refusal, Func<string?> write, Action revert, string focus = "")
    {
        FormLook.Say(refusal, null);
        var refused = await FormWrites.Off(write);
        if (refused is null)
        {
            host.Written(focus.Length > 0 ? focus : c.Path);
            return;
        }
        FormLook.Say(refusal, refused);
        host.Refused(c);
        revert();
    }

    private T Named<T>(T control, FormControl c, string key = "") where T : Control
    {
        AutomationProperties.SetName(control, MapUiJson.SentenceCase(c.Heading));
        host.Track(key.Length > 0 ? key : c.Path, control);
        return control;
    }

    private FrameworkElement ToggleRow(FormControl c, FormBlock block)
    {
        var (left, refusal, _) = Describe(c, block);
        var toggle = Named(new ToggleSwitch { IsOn = c.On, OnContent = "", OffContent = "", MinWidth = 0, IsEnabled = c.AcceptsWrite }, c);
        toggle.Toggled += (_, _) =>
        {
            var on = toggle.IsOn;
            if (on != c.On) Commit(c, refusal, () => FormWrites.Toggle(c, on), () => toggle.IsOn = c.On);
        };
        return FormLook.Row(left, toggle);
    }

    private FrameworkElement ChoiceRow(FormControl c, FormBlock block)
    {
        var (left, refusal, _) = Describe(c, block);
        var combo = Named(FormLook.Combo(c.Options.Select(o => MapUiJson.SentenceCase(o.Label)), c.ChoiceIndex, FormLook.ChoiceWidth,
            c.ChoiceIndex < 0 ? MapUiJson.SentenceCase(c.Display) : ""), c);
        combo.IsEnabled = c.AcceptsWrite;
        combo.SelectionChanged += async (_, _) =>
        {
            var index = combo.SelectedIndex;
            if (index < 0 || index == c.ChoiceIndex) return;
            var label = c.Options[index].Label;
            if (Dangerous(label) && !await FormLook.Confirm(combo.XamlRoot, $"{label}?", DangerNote, "Choose it"))
            {
                combo.SelectedIndex = c.ChoiceIndex;
                return;
            }
            Commit(c, refusal, () => FormWrites.Choose(c, index), () => combo.SelectedIndex = c.ChoiceIndex);
        };
        return FormLook.Row(left, combo);
    }

    private FrameworkElement NumberRow(FormControl c, FormBlock block)
    {
        var (left, refusal, hint) = Describe(c, block);
        var box = Named(new NumberBox
        {
            Value = c.Number ?? double.NaN,
            Width = FormLook.NumberWidth,
            SmallChange = c.Step,
            LargeChange = c.Step * 10,
            SpinButtonPlacementMode = NumberBoxSpinButtonPlacementMode.Compact,
            ValidationMode = NumberBoxValidationMode.InvalidInputOverwritten,
            IsEnabled = c.AcceptsWrite,
        }, c);
        box.ValueChanged += (_, args) =>
        {
            var typed = args.NewValue;
            if (double.IsNaN(typed))
            {
                if (c.Number is { } held) box.Value = held;
                return;
            }
            if (c.Number is { } now && Same(now, typed)) return;
            Commit(c, refusal, () => FormWrites.Number(c, typed), () => box.Value = c.Number ?? double.NaN);
        };
        box.GotFocus += (_, _) => FormLook.Say(hint, c.ConstraintNote);
        box.LostFocus += (_, _) => FormLook.Say(hint, null);
        return FormLook.Row(left, FormLook.Line(8, box, Units(c)));
    }

    private FrameworkElement TextRow(FormControl c, FormBlock block) => TextRow(c, block, c.ValueString, typed => typed);

    private FrameworkElement TextRow(FormControl c, FormBlock block, string shown, Func<string, string> stored)
    {
        var (left, refusal, _) = Describe(c, block);
        var held = new Held(shown);
        Control input = c.Secret
            ? new PasswordBox { Password = shown, PasswordRevealMode = PasswordRevealMode.Peek }
            : FormLook.Field(shown);
        input.IsEnabled = c.AcceptsWrite;
        string Typed() => input is PasswordBox secret ? secret.Password : ((TextBox)input).Text.Replace("\n", "");
        void Save()
        {
            var typed = Typed();
            if (typed == held.Text) return;
            held.Text = typed;
            Commit(c, refusal, () => FormWrites.Text(c, stored(typed)), () => held.Text = shown);
        }
        input.LostFocus += (_, _) => Save();
        input.KeyDown += (_, e) =>
        {
            if (e.Key == VirtualKey.Enter) Save();
        };
        return FormLook.Row(left, Units(c), Named(input, c));
    }

    private FrameworkElement BitsRow(FormControl c, FormBlock block)
    {
        var (left, refusal, _) = Describe(c, block, c.BitsSummary);
        var boxes = c.Bits.Select((bit, at) =>
        {
            var key = $"{c.Path}#{at}";
            var box = Named(new CheckBox
            {
                Content = bit.Label,
                IsChecked = c.BitSet(bit),
                IsEnabled = c.AcceptsWrite && c.BitEnabled(at),
                MinWidth = 0,
            }, c, key);
            FormLook.Toggled(box, on =>
            {
                if (on != c.BitSet(bit)) Commit(c, refusal, () => FormWrites.Bits(c, c.BitsToggled(at)), () => box.IsChecked = c.BitSet(bit), key);
            });
            return (UIElement)box;
        });
        return FormLook.Row(left, null, FormLook.Fill(new VariableSizedWrapGrid { Orientation = Orientation.Horizontal, ItemWidth = 220 }, boxes));
    }

    private FrameworkElement SliderRow(FormControl c, FormBlock block)
    {
        var slider = c.Slider!;
        var decimals = Math.Clamp(slider.Decimals, 0, 6);
        var (left, refusal, _) = Describe(c, block);
        var value = FormLook.Text("", "FontSizeBody", "AccentTextBrush", true);
        var bar = Named(new Slider
        {
            Minimum = slider.From,
            Maximum = slider.To,
            StepFrequency = c.Step,
            SmallChange = c.Step,
            Value = Math.Clamp(MapUiJson.Number(c.Value) ?? c.Number ?? slider.From, slider.From, slider.To),
            IsEnabled = c.AcceptsWrite,
            IsThumbToolTipEnabled = false,
        }, c);
        void Show() => FormLook.Say(value, string.Join(" ", new[]
        {
            Math.Round(bar.Value, decimals).ToString($"F{decimals}", CultureInfo.InvariantCulture),
            c.UnitsShown,
        }.Where(part => part.Length > 0)));
        var commit = new SliderCommit(bar, v => Math.Round(v, decimals), (chosen, revert) =>
            Commit(c, refusal, () => FormWrites.Set(c.Path, chosen), revert));
        Show();
        bar.ValueChanged += (_, _) => Show();
        Live.Watch(bar, c.LivePath, view =>
        {
            if ((MapUiJson.Number(view["value"]) ?? FormText.Parse(FormJson.Shown(view["valueString"]))) is { } pushed) commit.Show(pushed);
        });
        return FormLook.Row(left, value, FormLook.Stack(4, bar, FormLook.Caption(c.Description)));
    }

    private FrameworkElement ValueRow(FormControl c, FormBlock block, string note)
    {
        var (left, _, _) = Describe(c, block, note);
        return FormLook.Row(left, FormLook.Text(c.ShownValue, "FontSizeBody", "TextSecondaryBrush"));
    }

    private static FrameworkElement LabelRow(FormControl c) => FormLook.Margined(LabelText(c), new Thickness(16, 12, 16, 12));

    private static TextBlock LabelText(FormControl c)
    {
        var label = FormLook.Text(c.Title, c.SmallFont ? "FontSizeCaption" : "FontSizeBody", c.Warning ? "CriticalBrush" : "TextSecondaryBrush");
        label.Opacity = c.Enabled ? 1 : 0.38;
        return label;
    }

    private FrameworkElement ActionRow(FormControl c, bool dialog)
    {
        var refusal = FormLook.Caption("", "CriticalBrush");
        var button = Named(FormLook.Button(MapUiJson.SentenceCase(c.Title), () =>
        {
            if (dialog) host.RequestDialog(c);
            else Commit(c, refusal, () => FormWrites.Press(c), () => { });
        }), c);
        button.IsEnabled = c.Enabled;
        return FormLook.Margined(FormLook.Stack(4, button, refusal), new Thickness(16, 12, 16, 12));
    }

    private UIElement CalculatorRow(FormControl c, JsonObject calculator) => FormLook.Row(
        FormLook.Stack(2,
            FormLook.Text(MapUiJson.SentenceCase(MapUiJson.Text(calculator["title"]) is { Length: > 0 } title ? title : c.Heading)),
            FormLook.Caption($"Measure the {MapUiJson.Text(calculator["measure"])} with a meter")),
        FormLook.Button("Calibrate", () => host.RequestCalculator(c, calculator)));

    private static TextBlock Units(FormControl c)
    {
        var units = FormLook.Text(c.UnitsShown, "FontSizeBody", "TextSecondaryBrush");
        units.VerticalAlignment = VerticalAlignment.Center;
        return units;
    }

    private static bool Same(double a, double b) => Math.Abs(a - b) <= 1e-9 * Math.Max(1, Math.Abs(a));

    private sealed class Held(string text)
    {
        public string Text { get; set; } = text;
    }
}

internal static class AspectRatioText
{
    public const string Setting = "aspectRatio";

    private const double Tolerance = 0.005;
    private static readonly (int Width, int Height)[] Common = [(16, 9), (4, 3), (21, 9), (16, 10), (3, 2), (5, 4), (1, 1)];

    public static string Shown(FormControl c) =>
        Common.Where(r => c.Number is double value && Math.Abs((double)r.Width / r.Height - value) < Tolerance)
            .Select(r => $"{r.Width}:{r.Height}")
            .FirstOrDefault() ?? c.ValueString;

    public static string Stored(string typed) =>
        typed.Split(':') is [var width, var height] && FormText.Parse(width) is double w and > 0 && FormText.Parse(height) is double h and > 0
            ? (w / h).ToString("F6", CultureInfo.InvariantCulture)
            : typed;
}

internal sealed class SliderCommit
{
    private static readonly TimeSpan Settle = TimeSpan.FromMilliseconds(400);

    private readonly Slider _slider;
    private readonly Func<double, double> _round;
    private readonly Action<double, Action> _write;
    private readonly DispatcherTimer _settle = new() { Interval = Settle };
    private double _known;
    private bool _pressed;
    private bool _quiet;

    public SliderCommit(Slider slider, Func<double, double> round, Action<double, Action> write)
    {
        _slider = slider;
        _round = round;
        _write = write;
        _known = round(slider.Value);
        slider.AddHandler(UIElement.PointerPressedEvent, new PointerEventHandler((_, _) => _pressed = true), true);
        new[] { UIElement.PointerReleasedEvent, UIElement.PointerCaptureLostEvent, UIElement.PointerCanceledEvent }.ToList()
            .ForEach(routed => slider.AddHandler(routed, new PointerEventHandler((_, _) => Release()), true));
        slider.KeyUp += (_, _) => Commit();
        slider.ValueChanged += (_, _) =>
        {
            if (_quiet || _pressed) return;
            _settle.Stop();
            _settle.Start();
        };
        _settle.Tick += (_, _) => Commit();
        slider.Unloaded += (_, _) =>
        {
            if (_settle.IsEnabled) Commit();
        };
    }

    public void Show(double value)
    {
        if (_pressed || _settle.IsEnabled) return;
        _quiet = true;
        _slider.Value = Math.Clamp(value, _slider.Minimum, _slider.Maximum);
        _quiet = false;
        _known = _round(_slider.Value);
    }

    private void Release()
    {
        if (!_pressed) return;
        _pressed = false;
        Commit();
    }

    private void Commit()
    {
        _settle.Stop();
        var chosen = _round(_slider.Value);
        if (Math.Abs(chosen - _known) <= 1e-9 * Math.Max(1, Math.Abs(chosen))) return;
        var previous = _known;
        _known = chosen;
        _write(chosen, () => Show(previous));
    }
}
