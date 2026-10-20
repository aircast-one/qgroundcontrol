using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Pages.Setup;

internal sealed record ParameterTool(string Path, string Label, bool Confirm, string ConfirmTitle, string ConfirmMessage)
{
    public string ConfirmLabel => ConfirmTitle == "Reset All" ? "Reset" : "Ok";
}

internal sealed record ParameterDiff(JsonObject Json, string Name, string FileValue, string VehicleValue, string Units, bool CannotSend, int ComponentId, bool NoVehicleValue)
{
    public string Key => $"{ComponentId}:{Name}";

    public string Line =>
        CannotSend ? $"Vehicle N/A — not on Vehicle · File {FileValue}"
        : NoVehicleValue ? string.Join(" · ", new[] { "Vehicle N/A — new to Vehicle", $"File {FileValue}", Units }.Where(p => p.Trim().Length > 0))
        : string.Join(" · ", new[] { $"Vehicle {VehicleValue}", $"File {FileValue}", Units }.Where(p => p.Trim().Length > 0));
}

internal sealed record ParameterReview(IReadOnlyList<ParameterDiff> Rows, bool OtherVehicle, bool MultipleComponents, int Parsed, int Unchanged, int ReadOnly)
{
    public const string SendHint = "Click 'Ok' to update the parameters below on the Vehicle.";
    public const string CannotSendHint = "Parameters marked 'not on Vehicle' cannot be sent since the file does not include type information. They may only exist after another parameter is changed and the Vehicle is rebooted, after which you can load this file again.";
    public const string NewToVehicleHint = "Parameters marked 'new to Vehicle' have not been reported by the Vehicle. They may only become visible after they are sent and the Vehicle is rebooted.";

    public int Sendable => Rows.Count(r => !r.CannotSend);

    public IReadOnlyList<string> Warnings => new[]
    {
        OtherVehicle ? "The parameters in the file are from a different vehicle." : "",
        MultipleComponents ? "The file contains parameters for more than one component." : "",
    }.Where(w => w.Length > 0).ToList();

    public string Summary
    {
        get
        {
            var newToVehicle = Rows.Count(r => r.NoVehicleValue && !r.CannotSend);
            var missing = Rows.Count(r => r.CannotSend);
            var clauses = new[]
            {
                Sendable == 0 ? "" : newToVehicle > 0 ? $"{Sendable} will be changed (including {newToVehicle} not currently on the Vehicle)" : $"{Sendable} will be changed",
                Unchanged == 1 ? "1 already matches the Vehicle" : Unchanged > 1 ? $"{Unchanged} already match the Vehicle" : "",
                ReadOnly == 1 ? "1 read-only parameter will not be sent" : ReadOnly > 1 ? $"{ReadOnly} read-only parameters will not be sent" : "",
                missing > 0 ? $"{missing} not found on the Vehicle and cannot be sent" : "",
            }.Where(c => c.Length > 0).ToList();
            var loaded = Parsed == 1 ? "Loaded 1 parameter from file" : $"Loaded {Parsed} parameters from file";
            return clauses.Count == 0 ? $"{loaded}." : $"{loaded}: {string.Join(", ", clauses)}.";
        }
    }

    public static ParameterReview? Parse(JsonNode? node) => node is not JsonObject review ? null : new ParameterReview(
        MapUiJson.Objects(review["rows"]).Select(row => new ParameterDiff(
            row,
            MapUiJson.Text(row["name"]),
            FormJson.Shown(row["fileValue"]),
            FormJson.Shown(row["vehicleValue"]),
            MapUiJson.Text(row["units"]),
            MapUiJson.Flag(row["cannotSend"]),
            (int)(MapUiJson.Number(row["componentId"]) ?? 1),
            MapUiJson.Flag(row["noVehicleValue"]))).ToList(),
        MapUiJson.Flag(review["otherVehicle"]),
        MapUiJson.Flag(review["multipleComponents"]),
        (int)(MapUiJson.Number(review["parsed"]) ?? 0),
        (int)(MapUiJson.Number(review["unchanged"]) ?? 0),
        (int)(MapUiJson.Number(review["readOnly"]) ?? 0));
}

internal sealed class ParameterTools
{
    public const string View = "view.parameterTools";
    private const string Save = "parameterFile.save";
    private const string Review = "parameterFile.review";
    private const string Apply = "parameterFile.apply";
    private const string FileName = "vehicle.params";

    private readonly Action _refreshed;
    private IReadOnlyList<ParameterTool> _tools = [];

    public ParameterTools(Action refreshed)
    {
        _refreshed = refreshed;
        Button = new DropDownButton { Content = "Tools", FontFamily = PreflightLook.Font, Visibility = Visibility.Collapsed };
        Live.Watch(Button, View, Render);
    }

    public DropDownButton Button { get; }

    public static IReadOnlyList<ParameterTool> Parse(JsonObject view) =>
        MapUiJson.Objects(view["tools"]).Select(tool => new ParameterTool(
            MapUiJson.Text(tool["path"]),
            MapUiJson.Text(tool["label"]),
            MapUiJson.Flag(tool["confirm"]),
            MapUiJson.Text(tool["confirmTitle"]),
            MapUiJson.Text(tool["confirmMessage"]))).ToList();

    private void Render(JsonObject view)
    {
        _tools = Parse(view);
        var menu = new MenuFlyout();
        _tools.Select(tool =>
        {
            var item = new MenuFlyoutItem { Text = tool.Label };
            item.Click += (_, _) => Pick(tool);
            return item;
        }).ToList().ForEach(menu.Items.Add);
        Button.Flyout = menu;
        Button.Visibility = FormLook.Shown(_tools.Count > 0);
    }

    private async void Pick(ParameterTool tool)
    {
        if (tool.Confirm && !await FormLook.Confirm(Button.XamlRoot, tool.ConfirmTitle, tool.ConfirmMessage, tool.ConfirmLabel)) return;
        switch (tool.Path)
        {
            case Save: SaveFile(); break;
            case Review: LoadFile(); break;
            default:
                var path = tool.Path;
                var refused = await FormWrites.Off(() => SetupUi.Refusal(path));
                if (refused is null) _refreshed();
                else await Tell(refused);
                break;
        }
    }

    private async void SaveFile()
    {
        if (await FilePicks.Save(Button.XamlRoot, FileName, "Parameter file", ".params") is not { } target) return;
        var refused = await Task.Run(() =>
        {
            if (FormWrites.Result(Save) is not JsonValue value || !value.TryGetValue<string>(out var saved)) return "The parameters could not be read from the vehicle.";
            try
            {
                File.WriteAllText(target, saved);
                return null;
            }
            catch (Exception error) when (error is IOException or UnauthorizedAccessException) { return "The file could not be written."; }
        });
        if (refused is not null) await Tell(refused);
    }

    private async void LoadFile()
    {
        if (await FilePicks.Open(Button.XamlRoot, [FilePicks.Any]) is not { } source) return;
        var read = await Task.Run(() =>
        {
            try { return File.ReadAllText(source); }
            catch (Exception error) when (error is IOException or UnauthorizedAccessException) { return null; }
        });
        if (read is not { } text)
        {
            await Tell("Unable to open file.");
            return;
        }
        var reviewed = await Task.Run(() => Bridge.Invoke(Review, FormJson.Args(text)));
        if (ParameterReview.Parse(reviewed["result"]) is not { } review)
        {
            await Tell(MapUiJson.Text(reviewed["reason"]) is { Length: > 0 } reason ? reason : "The file could not be reviewed.");
            return;
        }
        ShowReview(review);
    }

    private async void ShowReview(ParameterReview review)
    {
        var boxes = review.Rows.Select(row =>
        {
            var box = new CheckBox
            {
                IsChecked = !row.CannotSend,
                IsEnabled = !row.CannotSend,
                Content = FormLook.Stack(2,
                    FormLook.Text(review.MultipleComponents ? $"{row.ComponentId}: {row.Name}" : row.Name),
                    FormLook.Caption(row.Line)),
            };
            return (Row: row, Box: box);
        }).ToList();
        var dialog = FormLook.Dialog(Button.XamlRoot, "Load Parameters", new ScrollViewer
        {
            MaxHeight = 520,
            Content = FormLook.Stack(8, review.Warnings.Select(w => (UIElement?)FormLook.Caption(w, "CautionBrush"))
                .Concat(
                [
                    FormLook.Text(review.Summary),
                    review.Sendable > 0 ? FormLook.Text(ParameterReview.SendHint) : null,
                    review.Rows.Count > 0 ? FormLook.Check("Name", true, on => boxes.Where(b => !b.Row.CannotSend).ToList().ForEach(b => b.Box.IsChecked = on)) : null,
                    FormLook.Stack(0, boxes.Select(b => (UIElement?)b.Box)),
                    review.Rows.Any(r => r.NoVehicleValue && !r.CannotSend) ? FormLook.Caption(ParameterReview.NewToVehicleHint) : null,
                    review.Rows.Any(r => r.CannotSend) ? FormLook.Caption(ParameterReview.CannotSendHint) : null,
                ])),
        }, review.Sendable > 0 ? "Ok" : null);
        void Paint() => dialog.IsPrimaryButtonEnabled = boxes.Any(b => b.Box.IsChecked == true);
        boxes.ForEach(b =>
        {
            b.Box.Checked += (_, _) => Paint();
            b.Box.Unchecked += (_, _) => Paint();
        });
        Paint();
        if (await FormLook.Show(dialog) != ContentDialogResult.Primary) return;
        var rows = new JsonArray(boxes.Where(b => b.Box.IsChecked == true).Select(b => (JsonNode?)b.Row.Json.DeepClone()).ToArray());
        var refused = await Task.Run(() => FormWrites.Refusal(Bridge.Invoke(Apply, new JsonArray(rows))));
        _refreshed();
        if (refused is not null) await Tell(refused);
    }

    private Task Tell(string text) => FormLook.Tell(Button.XamlRoot, "Parameters", text);
}
