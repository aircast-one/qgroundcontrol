using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Pages.Analyze;
using AircastQGC.Pages.Setup;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

internal sealed record BlockingCheck(VehicleMessage Message, string Description);

internal static class VehicleMessageRules
{
    internal const string WarningsPath = "view.warnings";
    internal const string ResetAll = "vehicle.resetAllMessages";
    internal const string Clear = "vehicle.clearMessages";

    internal static IReadOnlyList<VehicleMessage> Banner(IReadOnlyList<VehicleMessage> messages, int unread) =>
        messages.TakeLast(Math.Max(unread, 0))
            .Where(message => message.Level is "error" or "warning" && !message.ToldElsewhere)
            .ToList();

    internal static string BannerText(IReadOnlyList<VehicleMessage> shown) =>
        (shown.LastOrDefault(message => message.Level == "error") ?? shown.LastOrDefault(message => message.Level == "warning"))
            ?.Text is { Length: > 0 } worst
            ? worst
            : $"{shown.Count} message{(shown.Count == 1 ? "" : "s")} from the vehicle";

    internal static IReadOnlyList<BlockingCheck> Blocking(JsonObject warnings)
    {
        var checks = MapUiJson.Objects(warnings["armingChecks"])
            .Where(check => MapUiJson.Text(check["message"]).Trim().Length > 0)
            .Select(check => new BlockingCheck(
                new VehicleMessage("", MapUiJson.Text(check["severity"]) == "error" ? "error" : "warning", MapUiJson.Text(check["message"])),
                MapUiJson.Text(check["description"]).Trim()))
            .ToList();
        return checks.Count > 0
            ? checks
            : MapUiJson.Text(warnings["armingBlocker"]).Trim() is { Length: > 0 } blocker
                ? [new BlockingCheck(new VehicleMessage("", "error", blocker), "")]
                : [];
    }
}

public sealed class VehicleMessageBanner : UserControl
{
    private readonly string _client = $"fly.messages.{Guid.NewGuid():N}";
    private readonly FontIcon _icon = Ui.Glyph("", 20, "CautionBrush");
    private readonly TextBlock _text = Ui.Text("", semibold: true);
    private readonly Border _card;
    private IReadOnlyList<VehicleMessage> _messages = [];
    private ContentDialog? _sheet;
    private ItemsControl? _sheetLines;
    private TextBlock? _sheetSummary;
    private JsonObject _warnings = new();
    private string? _editing;

    public VehicleMessageBanner()
    {
        _text.MaxLines = 2;
        _text.TextTrimming = TextTrimming.CharacterEllipsis;
        var open = Ui.Styled(new Button
        {
            BorderThickness = new Thickness(0),
            Padding = new Thickness(12, 6, 4, 6),
            HorizontalAlignment = HorizontalAlignment.Stretch,
            HorizontalContentAlignment = HorizontalAlignment.Left,
            Content = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 10, Children = { _icon, _text } },
        }.Named("Vehicle messages"), Look.Subtle, OpenSheet);
        _card = new Border
        {
            CornerRadius = (CornerRadius)Application.Current.Resources["RadiusOverlay"],
            MinHeight = 40,
            MaxWidth = 560,
            Child = new Grid
            {
                ColumnDefinitions =
                {
                    new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) },
                    new ColumnDefinition { Width = GridLength.Auto },
                },
                Children =
                {
                    open,
                    Ui.IconButton("", "Dismiss messages", () => Ui.Off(() => Bridge.Invoke(VehicleMessageRules.ResetAll)))
                        .With(button =>
                        {
                            button.Margin = new Thickness(0, 0, 4, 0);
                            Grid.SetColumn(button, 1);
                        }),
                },
            },
        };
        Content = _card;
        Visibility = Visibility.Collapsed;
        Loaded += (_, _) =>
        {
            BridgeWatch.Watch(_client, [MessagesTool.MessagesPath], Apply);
            Ui.Off(this, () => Bridge.Get(MessagesTool.MessagesPath), Apply);
        };
        Unloaded += (_, _) => BridgeWatch.Stop(_client);
    }

    private void Apply(JsonObject view)
    {
        _messages = MessagesTool.Parse(view);
        var shown = VehicleMessageRules.Banner(_messages, MessagesTool.Unread(view));
        Visibility = Ui.Shown(shown.Count > 0);
        RenderSheet();
        if (shown.Count == 0) return;
        var urgent = shown.Any(message => message.Level == "error");
        _card.Background = Ui.Token(urgent ? "CriticalBgBrush" : "CautionBgBrush");
        _icon.Glyph = urgent ? "" : "";
        _icon.Foreground = Ui.Token(urgent ? "CriticalBrush" : "CautionBrush");
        _text.Text = VehicleMessageRules.BannerText(shown);
    }

    internal async void OpenSheet()
    {
        if (_sheet != null || XamlRoot is not { } root) return;
        Ui.Off(() => Bridge.Invoke(VehicleMessageRules.ResetAll));
        _sheetSummary = Ui.Text("", brush: "TextSecondaryBrush");
        _sheetLines = Ui.Column([]);
        _sheet = new ContentDialog
        {
            XamlRoot = root,
            Title = "Messages",
            PrimaryButtonText = "Clear",
            CloseButtonText = "Close",
            DefaultButton = ContentDialogButton.Close,
            Content = new StackPanel
            {
                Spacing = 12,
                Children =
                {
                    _sheetSummary,
                    new ScrollViewer { MaxHeight = 420, Content = _sheetLines },
                },
            },
        };
        RenderSheet();
        Ui.Off(this, () => Bridge.Get(VehicleMessageRules.WarningsPath), warnings =>
        {
            _warnings = warnings;
            RenderSheet();
        });
        try
        {
            if (await _sheet.ShowAsync() == ContentDialogResult.Primary) Ui.Off(() => Bridge.Invoke(VehicleMessageRules.Clear));
        }
        catch (COMException)
        {
        }
        _sheet = null;
        _sheetLines = null;
        _sheetSummary = null;
        if (_editing is not { } name) return;
        _editing = null;
        ParameterEditor.Show(root, name, false, OpenSheet);
    }

    private void EditParameter(string name)
    {
        _editing = name;
        _sheet?.Hide();
    }

    private UIElement CheckLine(BlockingCheck check) =>
        MessagesTool.Line(check.Message, check.Description.Length > 0 ? ParameterLinks.Text(check.Description, EditParameter) : new Border());

    private void RenderSheet()
    {
        if (_sheet == null || _sheetLines == null || _sheetSummary == null) return;
        var blocking = VehicleMessageRules.Blocking(_warnings);
        var lines = _messages.Reverse().ToList();
        _sheet.Title = blocking.Count == 0 ? "Messages" : "Why it will not arm";
        _sheetSummary.Text = MessagesTool.Summary(lines);
        _sheetLines.Fill(lines.Count == 0 && blocking.Count == 0
            ? [Ui.Text("The vehicle has not said anything yet.", brush: "TextSecondaryBrush")]
            : blocking.Select(CheckLine).Concat(lines.Select(MessagesTool.Line)));
    }
}
