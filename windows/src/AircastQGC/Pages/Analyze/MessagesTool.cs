using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;
using AircastQGC.Controls;
using AircastQGC.Core;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Pages.Analyze;

internal sealed record VehicleMessage(string Time, string Level, string Text, bool ToldElsewhere = false);

internal sealed class MessagesTool : UserControl
{
    internal const string MessagesPath = "view.messages";
    private readonly string _client = $"analyzeTool.messages.{System.Guid.NewGuid():N}";

    private readonly ItemsControl _lines = Ui.Column([]);
    private readonly ScrollViewer _list;
    private readonly UIElement _empty = Ui.EmptyState("\uE8A5", "No messages", "The vehicle has not said anything yet.");

    public MessagesTool()
    {
        _list = new ScrollViewer
        {
            Content = new StackPanel
            {
                Padding = new Thickness(24, 0, 24, 24),
                Children =
                {
                    _lines,
                    Ui.Button("Clear", () => Ui.Off(() => Bridge.Invoke("vehicle.clearMessages")), Look.Subtle)
                        .With(button => button.HorizontalAlignment = HorizontalAlignment.Right),
                },
            },
        };
        Content = _empty;
        Loaded += (_, _) =>
        {
            BridgeWatch.Watch(_client, [MessagesPath], Apply);
            Apply(Bridge.Group(MessagesPath));
            Ui.Off(() => Bridge.Invoke("vehicle.resetAllMessages"));
        };
        Unloaded += (_, _) => BridgeWatch.Stop(_client);
    }

    private void Apply(JsonObject view)
    {
        var lines = Parse(view).Reverse().ToList();
        Content = lines.Count == 0 ? _empty : _list;
        _lines.Fill(lines.Select(Line));
    }

    internal static int Unread(JsonObject view) => Ui.Int(view["unread"]);

    internal static IReadOnlyList<VehicleMessage> Parse(JsonObject view)
    {
        var read = MapUiJson.Objects(view["items"])
            .Select(item => new VehicleMessage(Time(MapUiJson.Text(item["time"])), MapUiJson.Text(item["level"]), MapUiJson.Text(item["text"]), MapUiJson.Flag(item["toldElsewhere"])))
            .Where(message => !string.IsNullOrWhiteSpace(message.Text))
            .ToList();
        return MapUiJson.Text(view["order"]) == "oldestFirst" ? read : read.AsEnumerable().Reverse().ToList();
    }

    internal static string Summary(IReadOnlyList<VehicleMessage> messages)
    {
        static string Counted(int n, string one) => $"{n} {one}{(n == 1 ? "" : "s")}";
        var errors = messages.Count(m => m.Level == "error");
        var warnings = messages.Count(m => m.Level == "warning");
        var rest = messages.Count - errors - warnings;
        var parts = new[]
        {
            errors > 0 ? Counted(errors, "error") : null,
            warnings > 0 ? Counted(warnings, "warning") : null,
            rest > 0 ? Counted(rest, "message") : null,
        }.OfType<string>().ToList();
        return parts.Count == 0 ? "0 messages from the vehicle" : string.Join(" · ", parts);
    }

    private static string Time(string served) =>
        Regex.Match(served.Trim(), @"^(\d{1,2}:\d{2}:\d{2})\.\d+$") is { Success: true } clock ? clock.Groups[1].Value : served;

    internal static UIElement Line(VehicleMessage message) => new Grid
    {
        ColumnSpacing = 16,
        Padding = new Thickness(0, 10, 0, 10),
        ColumnDefinitions =
        {
            new ColumnDefinition { Width = GridLength.Auto },
            new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) },
        },
        Children =
        {
            Ui.Glyph(message.Level switch { "error" => "\uE783", "warning" => "\uE7BA", _ => "\uE73E" }, 20,
                message.Level switch { "error" => "CriticalBrush", "warning" => "CautionBrush", _ => "SuccessBrush" })
                .With(icon => icon.VerticalAlignment = VerticalAlignment.Top),
            new StackPanel
            {
                Children =
                {
                    Ui.Caption(message.Time, "TextTertiaryBrush").With(time => time.Visibility = Ui.Shown(message.Time.Length > 0)),
                    Ui.Text(message.Text),
                },
            }.With(panel => Grid.SetColumn(panel, 1)),
        },
    };
}
