using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Shapes;

namespace AircastQGC.Pages.Setup;

internal sealed class ServosPanel : UserControl
{
    private const string View = "view.apmServos";
    private static readonly TimeSpan Poll = TimeSpan.FromMilliseconds(200);
    private static readonly IReadOnlyList<(string Key, string Title)> Fields = [("function", "Function"), ("min", "Min"), ("trim", "Trim"), ("max", "Max"), ("reversed", "Reversed")];

    private readonly FormView _form;
    private readonly Dictionary<int, (ProgressBar Bar, TextBlock Pwm)> _live = [];
    private JsonObject _latest = new();
    private string _signature = "";
    private bool _read;

    public ServosPanel()
    {
        _form = SetupUi.Form(() => Facts(_latest), Sections, new FormHooks { Replace = (form, section) => Servo(form, section) });
        Content = SetupUi.Notice("Reading servo outputs.");
        _ = Live.Poll(this, Poll, () => Bridge.Get(View), Apply);
    }

    private static JsonObject Facts(JsonObject view) =>
        new() { ["servos"] = new JsonArray(MapUiJson.Objects(view["servos"]).Select(servo => (JsonNode?)new JsonObject(servo
            .Where(kv => kv.Key is not ("pwm" or "position"))
            .Select(kv => KeyValuePair.Create(kv.Key, kv.Value?.DeepClone())))).ToArray()) };

    private static IReadOnlyList<FormSection> Sections(JsonObject facts) =>
        MapUiJson.Objects(facts["servos"]).Select(servo => SetupUi.Section(
            $"Servo {(int)(MapUiJson.Number(servo["index"]) ?? 0)}",
            "",
            SetupUi.Block("", Fields.Select(f => SetupUi.Control(servo[f.Key]) is { } control ? SetupUi.Titled(control, f.Title) : null).ToArray())))
            .ToList();

    private void Apply(JsonObject view)
    {
        _latest = view;
        var servos = MapUiJson.Objects(view["servos"]);
        if (!_read)
        {
            _read = true;
            Content = servos.Count == 0 ? SetupUi.Notice("This vehicle exposes no servo outputs.")
                : SetupUi.Page(FormLook.Text("Configure ArduPilot servo outputs.", "FontSizeBody", "TextSecondaryBrush"), _form);
        }
        servos.ToList().ForEach(servo =>
        {
            if (!_live.TryGetValue((int)(MapUiJson.Number(servo["index"]) ?? 0), out var live)) return;
            live.Bar.Value = Math.Clamp(MapUiJson.Number(servo["position"]) ?? 0, 0, 1);
            live.Pwm.Text = MapUiJson.Number(servo["pwm"]) is { } pwm ? ((int)pwm).ToString(CultureInfo.InvariantCulture) : "-";
        });
        var signature = Facts(view).ToJsonString();
        if (signature == _signature) return;
        _signature = signature;
        _form.Reload();
    }

    private UIElement Servo(FormView form, FormSection section)
    {
        var index = int.TryParse(section.Title.Replace("Servo ", ""), NumberStyles.Integer, CultureInfo.InvariantCulture, out var n) ? n : 0;
        var bar = SetupUi.Bar(0);
        var pwm = FormLook.Text("-", "FontSizeBody", "TextSecondaryBrush");
        pwm.Width = 48;
        pwm.VerticalAlignment = VerticalAlignment.Center;
        _live[index] = (bar, pwm);
        var position = new Grid
        {
            ColumnSpacing = 12,
            Padding = new Thickness(16, 10, 16, 10),
            ColumnDefinitions = { new ColumnDefinition { Width = GridLength.Auto }, new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto } },
        };
        var label = FormLook.Text("Position");
        label.VerticalAlignment = VerticalAlignment.Center;
        position.Children.Add(label);
        Grid.SetColumn(bar, 1);
        position.Children.Add(bar);
        Grid.SetColumn(pwm, 2);
        position.Children.Add(pwm);
        return FormLook.Stack(4, FormLook.Card(position), form.BlockCard(section.Blocks[0]));
    }
}

internal sealed class FollowPanel : UserControl
{
    private const string View = "view.apmFollow";
    private const string Enable = "apmFollow.enable";
    private const string Reset = "apmFollow.reset";
    private const string Position = "apmFollow.position";
    private const string Point = "apmFollow.point";
    private const string Offsets = "apmFollow.offsets";
    private const string HeightPath = "apmFollow.height";
    private const double Graphic = 240;
    private static readonly TimeSpan WaitingPoll = TimeSpan.FromMilliseconds(500);

    private readonly DispatcherTimer _waiting = new() { Interval = WaitingPoll };
    private JsonObject _view = new();
    private bool _loaded;
    private string? _refusal;
    private int _reads;

    public FollowPanel()
    {
        _waiting.Tick += (_, _) => Refresh();
        Loaded += (_, _) => Refresh();
        Unloaded += (_, _) => _waiting.Stop();
    }

    private async void Refresh()
    {
        var read = ++_reads;
        var view = await FormWrites.Off(() => Bridge.Get(View));
        if (read != _reads) return;
        _view = view;
        _loaded = true;
        if (MapUiJson.Flag(view["waiting"])) _waiting.Start();
        else _waiting.Stop();
        Render();
    }

    private double Number(string key) => MapUiJson.Number(_view[key]) ?? 0;

    private (string Name, double PerUnit) Unit(string axis) =>
        (MapUiJson.Text(_view[$"{axis}Unit"]) is { Length: > 0 } name ? name : "m",
         MapUiJson.Number(_view[$"{axis}MetresPerUnit"]) is { } per && per > 0 ? per : 1);

    private static string OneDecimal(double value) => value.ToString("F1", CultureInfo.InvariantCulture);

    private void Render()
    {
        if (!_loaded) return;
        if (!MapUiJson.Flag(_view["available"]))
        {
            Content = SetupUi.Notice("This vehicle has no follow me parameters.");
            return;
        }
        var rover = MapUiJson.Flag(_view["rover"]);
        var positionIndex = (int)Number("positionIndex");
        var horizontal = Unit("horizontal");
        var vertical = Unit("vertical");
        var angle = Number("angle");
        var distance = Number("distance");
        var height = Number("height");
        var reset = FormLook.Button("Reset to supported settings", () => Act(Reset), ButtonLook.Accent);
        Content = SetupUi.Page(new UIElement?[]
        {
            FormLook.Check("Enable follow me", MapUiJson.Flag(_view["enabled"]), on => Act(Enable, on)),
            MapUiJson.Flag(_view["waiting"]) ? FormLook.Text("Waiting for Vehicle to update") : null,
            MapUiJson.Flag(_view["supported"], true) ? null : FormLook.Text(MapUiJson.Text(_view["unsupportedText"]), "FontSizeBody", "CriticalBrush"),
            MapUiJson.Flag(_view["supported"], true) ? null : reset,
        }
        .Concat(MapUiJson.Flag(_view["showSettings"])
            ? new UIElement?[]
            {
                SetupUi.Heading("Follow me settings"),
                FormLook.Card(new UIElement?[]
                {
                    SetupUi.Choice("Vehicle position", MapUiJson.Texts(_view["positionOptions"]), positionIndex, at => Act(Position, at)),
                    rover ? null : SetupUi.Choice("Point vehicle", MapUiJson.Texts(_view["pointOptions"]), (int)(MapUiJson.Number(_view["pointIndex"]) ?? -1), at => Act(Point, at)),
                }.OfType<UIElement>()),
            }
            .Concat(positionIndex == 1
                ? [
                    SetupUi.Heading("Vehicle offsets"),
                    FormLook.Caption("Click in the graphic to change angle"),
                    FormLook.Line(16, Offset(angle, distance, horizontal, rover), rover ? null : HeightGraphic($"{OneDecimal(height / vertical.PerUnit)} {vertical.Name}")),
                    FormLook.Card(new UIElement?[]
                    {
                        SetupUi.Entry("Angle", OneDecimal(angle), typed => { if (FormText.Parse(typed) is { } value) Act(Offsets, value, distance); }, units: "deg"),
                        SetupUi.Entry("Distance", OneDecimal(distance / horizontal.PerUnit), typed => { if (FormText.Parse(typed) is { } value) Act(Offsets, angle, value * horizontal.PerUnit); }, units: horizontal.Name),
                        rover ? null : SetupUi.Entry("Height", OneDecimal(height / vertical.PerUnit), typed => { if (FormText.Parse(typed) is { } value) Act(HeightPath, value * vertical.PerUnit); }, units: vertical.Name),
                    }.OfType<UIElement>()),
                ]
                : [])
            : [])
        .Append(_refusal is null ? null : FormLook.Text(_refusal, "FontSizeBody", "CriticalBrush"))
        .ToArray());
    }

    public static double HeadingOfTap(double x, double y) =>
        (90 - Math.Atan2(y, x) * 180 / Math.PI) is var heading && heading < 0 ? heading + 360 : heading > 360 ? heading - 360 : heading;

    private double VehicleYaw(bool rover) =>
        rover || (int)(MapUiJson.Number(_view["pointIndex"]) ?? 0) == 0 ? 0
        : (int)(MapUiJson.Number(_view["pointIndex"]) ?? 0) == 1 ? 180
        : -Number("angle");

    private UIElement Offset(double angle, double distance, (string Name, double PerUnit) unit, bool rover)
    {
        var center = Graphic / 2;
        var canvas = new Canvas { Width = Graphic, Height = Graphic, Background = FormLook.Brush("CardBrush") };
        canvas.Children.Add(new Line { X1 = center, X2 = center, Y1 = 0, Y2 = Graphic, Stroke = FormLook.Brush("DividerBrush"), StrokeThickness = 3 });
        canvas.Children.Add(new Line { X1 = 0, X2 = Graphic, Y1 = center, Y2 = center, Stroke = FormLook.Brush("DividerBrush"), StrokeThickness = 3 });
        canvas.Children.Add(Triangle(center, center, 14, 0, "AccentBrush"));
        var radians = angle * Math.PI / 180;
        var reach = center - 20;
        var (vx, vy) = (center + reach * Math.Sin(radians), center - reach * Math.Cos(radians));
        canvas.Children.Add(new Line
        {
            X1 = center + (reach - 14) * Math.Sin(radians),
            Y1 = center - (reach - 14) * Math.Cos(radians),
            X2 = center + 18 * Math.Sin(radians),
            Y2 = center - 18 * Math.Cos(radians),
            Stroke = FormLook.Brush("TextTertiaryBrush"),
            StrokeThickness = 2,
        });
        canvas.Children.Add(Triangle(vx, vy, 12, angle + VehicleYaw(rover), "TextPrimaryBrush"));
        var label = FormLook.Caption($"{OneDecimal(distance / unit.PerUnit)} {unit.Name}", "TextPrimaryBrush");
        var labelRadius = (center - 16) / 2;
        Canvas.SetLeft(label, center + labelRadius * Math.Sin(radians) - 20);
        Canvas.SetTop(label, center - labelRadius * Math.Cos(radians) - 8);
        canvas.Children.Add(label);
        AutomationProperties.SetName(canvas, "Follow offset angle");
        canvas.Tapped += (_, args) =>
        {
            var at = args.GetPosition(canvas);
            Act(Offsets, HeadingOfTap(at.X - center, center - at.Y), distance);
        };
        return canvas;
    }

    private static Polygon Triangle(double x, double y, double size, double rotation, string brush) => new()
    {
        Points = new PointCollection
        {
            new Windows.Foundation.Point(x, y - size),
            new Windows.Foundation.Point(x - size * 0.7, y + size * 0.7),
            new Windows.Foundation.Point(x + size * 0.7, y + size * 0.7),
        },
        Fill = FormLook.Brush(brush),
        RenderTransform = new RotateTransform { Angle = rotation, CenterX = x, CenterY = y },
    };

    private static UIElement HeightGraphic(string text)
    {
        var canvas = new Canvas { Width = 56, Height = Graphic };
        canvas.Children.Add(new Line { X1 = 28, X2 = 28, Y1 = 0, Y2 = Graphic, Stroke = FormLook.Brush("TextTertiaryBrush"), StrokeThickness = 2 });
        canvas.Children.Add(new Line { X1 = 20, X2 = 36, Y1 = 1, Y2 = 1, Stroke = FormLook.Brush("TextPrimaryBrush"), StrokeThickness = 2 });
        canvas.Children.Add(new Line { X1 = 20, X2 = 36, Y1 = Graphic - 1, Y2 = Graphic - 1, Stroke = FormLook.Brush("TextPrimaryBrush"), StrokeThickness = 2 });
        var label = new Border { Background = FormLook.Brush("BgBaseBrush"), Child = FormLook.Caption(text, "TextPrimaryBrush") };
        Canvas.SetLeft(label, 4);
        Canvas.SetTop(label, Graphic / 2 - 8);
        canvas.Children.Add(label);
        return canvas;
    }

    private void Act(string path, params object?[] args) => SetupUi.Run(() => SetupUi.Refusal(path, args), refused =>
    {
        _refusal = refused;
        Refresh();
    });
}
