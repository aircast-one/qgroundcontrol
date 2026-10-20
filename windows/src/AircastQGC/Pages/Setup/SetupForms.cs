using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Documents;

namespace AircastQGC.Pages.Setup;

internal static class SetupForms
{
    public static UIElement Page(string page, string? section, bool power)
    {
        var form = Hooked(FormView.SetupPage(page, section));
        return SetupUi.Scrolled(FormLook.Stack(0, power ? new PowerLiveCard() : null, form));
    }

    public static FormView Hooked(FormView form)
    {
        form.DialogRequested += (_, _) => EscCalibration.Show(form.XamlRoot, form.Reload);
        form.CalculatorRequested += (_, ask) => PowerCalc.Show(form.XamlRoot, ask.Calculator, form.Reload);
        return form;
    }

    public static FormView Plain(string page) =>
        SetupUi.Form(() => Bridge.Get($"view.setup({page})"), FormParse.SetupSections);

    public static FormView Opened(string page, Lazy<bool> opened, IReadOnlySet<string> highlighted) =>
        Hooked(new FormView(
            () =>
            {
                _ = opened.Value;
                return Bridge.Get($"view.setup({page})");
            },
            FormParse.SetupSections)
        {
            LoadingText = "Reading parameters from the vehicle.",
            NotesFirst = true,
            Highlighted = highlighted,
            Empty = () => FormLook.Empty("", "Nothing to set here", "This vehicle exposes none of these parameters."),
        });

    public static Lazy<bool> OpenedOnce(string page) => new(() => FormWrites.Ok(SetupModel.PageOpened, page));
}

internal static class EscCalibration
{
    private const string View = "view.escCalibration";
    private const string Start = "escCalibration.start";
    private const string Close = "escCalibration.close";
    private static readonly TimeSpan Every = TimeSpan.FromMilliseconds(500);

    public static async void Show(XamlRoot? root, Action done)
    {
        if (root is null) return;
        var text = FormLook.Text("", "FontSizeBody", "TextPrimaryBrush");
        var dialog = FormLook.Dialog(root, "ESC Calibration", text, "OK", "");
        dialog.IsPrimaryButtonEnabled = false;
        var timer = new DispatcherTimer { Interval = Every };
        var failed = new Failure();
        void Paint(JsonObject view)
        {
            if (failed.Reason is { } reason)
            {
                text.Inlines.Clear();
                text.Inlines.Add(new Run { Text = reason, Foreground = FormLook.Brush("CriticalBrush") });
                dialog.IsPrimaryButtonEnabled = true;
                return;
            }
            var open = MapUiJson.Flag(view["open"]);
            text.Inlines.Clear();
            text.Inlines.Add(new Run { Text = open ? MapUiJson.Text(view["highlight"]) : "", FontWeight = FontWeights.Bold, Foreground = FormLook.Brush("CriticalBrush") });
            text.Inlines.Add(new Run { Text = open ? MapUiJson.Text(view["text"]) : "Starting ESC calibration..." });
            text.Visibility = Visibility.Visible;
            dialog.IsPrimaryButtonEnabled = open && !MapUiJson.Flag(view["running"]);
        }
        Paint(new JsonObject());
        timer.Tick += (_, _) => SetupUi.Run(() => Bridge.Get(View), Paint);
        dialog.Closing += (_, args) => args.Cancel = args.Result != ContentDialogResult.Primary;
        dialog.Opened += (_, _) =>
        {
            SetupUi.Run(() => SetupUi.Refusal(Start), refused =>
            {
                failed.Reason = refused;
                if (refused is not null) Paint(new JsonObject());
            });
            timer.Start();
        };
        dialog.Closed += (_, _) => timer.Stop();
        if (await FormLook.Show(dialog) == ContentDialogResult.Primary && failed.Reason is null) SetupUi.Detached(() => FormWrites.Ok(Close));
        done();
    }

    private sealed class Failure
    {
        public string? Reason { get; set; }
    }
}

internal static class PowerCalc
{
    private const string View = "view.powerCalc";
    private const string Calculate = "vehicleConfig.calculate";
    private static readonly TimeSpan Every = TimeSpan.FromSeconds(1);

    public static async void Show(XamlRoot? root, JsonObject calculator, Action done)
    {
        if (root is null) return;
        var param = MapUiJson.Text(calculator["param"]);
        var measure = MapUiJson.Text(calculator["measure"]);
        var battery = (int)(MapUiJson.Number(calculator["batteryIndex"]) ?? 0);
        var noReadingText = MapUiJson.Text(calculator["noReading"]);
        var path = $"{View}({measure},{battery},{param})";
        var measured = FormLook.Field("", MapUiJson.Text(calculator["measuredLabel"]));
        var noReading = FormLook.Caption("", "CriticalBrush");
        var reading = FormLook.Text("", "FontSizeBody", "TextPrimaryBrush");
        var current = FormLook.Text("", "FontSizeBody", "TextPrimaryBrush");
        var refusal = SetupUi.Error();
        var dialog = FormLook.Dialog(root, MapUiJson.SentenceCase(MapUiJson.Text(calculator["title"])), FormLook.Stack(8,
            FormLook.Caption(MapUiJson.Text(calculator["help"])),
            noReading,
            measured,
            reading,
            current,
            refusal), MapUiJson.SentenceCase(MapUiJson.Text(calculator["button"])), "Close");
        void Paint(JsonObject live)
        {
            var available = MapUiJson.Flag(live["readingAvailable"]);
            FormLook.Say(noReading, !available && noReadingText.Length > 0 ? noReadingText : null);
            FormLook.Say(reading, available || noReadingText.Length == 0 ? $"{MapUiJson.Text(calculator["readingLabel"])} {MapUiJson.Text(live["readingText"])}" : null);
            FormLook.Say(current, $"{MapUiJson.Text(calculator["paramLabel"])} {MapUiJson.Text(live["paramText"])}");
            dialog.IsPrimaryButtonEnabled = available;
        }
        Paint(new JsonObject());
        var timer = new DispatcherTimer { Interval = Every };
        timer.Tick += (_, _) => SetupUi.Run(() => Bridge.Get(path), Paint);
        dialog.Opened += (_, _) =>
        {
            timer.Start();
            SetupUi.Run(() => Bridge.Get(path), Paint);
        };
        dialog.Closed += (_, _) => timer.Stop();
        dialog.PrimaryButtonClick += (_, args) =>
        {
            args.Cancel = true;
            var typed = measured.Text;
            SetupUi.Run(() => SetupUi.Refusal(Calculate, param, measure, battery, typed), refused => FormLook.Say(refusal, refused));
        };
        await FormLook.Show(dialog);
        done();
    }
}

internal sealed class PowerLiveCard : UserControl
{
    private static readonly IReadOnlyList<(string Name, string Label)> Tiles = [("voltage", "VOLTAGE"), ("current", "CURRENT"), ("mahConsumed", "USED")];

    public PowerLiveCard()
    {
        Margin = new Thickness(0, 16, 0, 0);
        Live.Watch(this, "view.battery", Render);
    }

    public static IReadOnlyList<(string Label, string Value, string Units)> Read(JsonObject view)
    {
        var facts = MapUiJson.Flag(view["available"])
            ? MapUiJson.Objects(MapUiJson.Objects(view["packs"]).FirstOrDefault()?["facts"]).ToDictionary(f => MapUiJson.Text(f["name"]), f => f)
            : new Dictionary<string, JsonObject>();
        return Tiles
            .Where(tile => facts.ContainsKey(tile.Name))
            .Select(tile => (tile.Label, MapUiJson.Text(facts[tile.Name]["valueString"]), MapUiJson.Text(facts[tile.Name]["units"]) is "v" ? "V" : MapUiJson.Text(facts[tile.Name]["units"])))
            .ToList();
    }

    private void Render(JsonObject view)
    {
        var tiles = Read(view);
        Content = tiles.Count == 0 ? null : new Border
        {
            Background = FormLook.Brush("CardBrush"),
            BorderBrush = FormLook.Brush("CardStrokeBrush"),
            BorderThickness = new Thickness(1),
            CornerRadius = FormLook.Radius("RadiusOverlay"),
            Padding = new Thickness(16),
            Child = FormLook.Fill(new Grid
            {
                ColumnSpacing = 16,
                ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition(), new ColumnDefinition() },
            }, tiles.Select((tile, at) =>
            {
                var value = FormLook.Line(4,
                    FormLook.Text(tile.Value, "FontSizeSubtitle", "TextPrimaryBrush", true),
                    FormLook.Margined(FormLook.Caption(tile.Units), new Thickness(0, 0, 0, 3)));
                value.VerticalAlignment = VerticalAlignment.Bottom;
                var column = FormLook.Stack(2, FormLook.Caption(tile.Label, "TextTertiaryBrush"), value);
                Grid.SetColumn(column, at);
                return (UIElement)column;
            })),
        };
    }
}
