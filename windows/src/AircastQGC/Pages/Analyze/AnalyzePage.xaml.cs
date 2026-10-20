using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;
using AircastQGC.Core;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Pages.Analyze;

internal sealed record AnalyzeTool(string Key, string Label, string Description, string Section, string Glyph, bool RequiresVehicle = false)
{
    internal const string FlightData = "Flight data";
    internal const string Live = "Live";
    internal const string Vehicle = "Vehicle";

    internal static readonly AnalyzeTool LogDownload = new("LogDownload", "Flight logs", "Download flight logs from the vehicle", FlightData, "\uE896", true);
    internal static readonly AnalyzeTool GeoTag = new("GeoTag", "Geotag images", "Match photos to the flight log", FlightData, "\uE722");
    internal static readonly AnalyzeTool Vibration = new("Vibration", "Vibration", "Accelerometer vibration levels and clipping", Live, "\uE877", true);
    internal static readonly AnalyzeTool Inspector = new("Inspector", "MAVLink inspector", "Live message rates and field values", Live, "\uE9D2", true);
    internal static readonly AnalyzeTool Console = new("Console", "Console", "Vehicle shell over MAVLink", Live, "\uE756", true);
    internal static readonly AnalyzeTool Messages = new("Messages", "Messages", "What the vehicle has said since it connected", Live, "\uE8BD");
    internal static readonly AnalyzeTool Firmware = new("Firmware", "Firmware", "Flash a board through its bootloader over USB", Vehicle, "\uEC7A");

    internal static readonly IReadOnlyList<AnalyzeTool> All = [LogDownload, GeoTag, Vibration, Inspector, Console, Messages, Firmware];
}

internal enum RowState { Attention, Done, Neutral }

internal sealed record ToolRow(AnalyzeTool Tool, Button Button, Border Marker, Border Badge, FontIcon Glyph, TextBlock Title, TextBlock Subtitle, TextBlock Status);

public sealed partial class AnalyzePage : UserControl
{
    internal const string RequiresVehicle = "Requires a connected vehicle";
    internal const string VehiclesPath = "view.vehicles";

    private const string FlyStateClient = "analyzePage.flyState";
    private const string VehiclesClient = "analyzePage.vehicles";
    private const string MessagesClient = "analyzePage.messages";
    private const string VibrationClient = "analyzePage.vibration";
    private const string InspectorClient = "analyzePage.inspector";
    private const double TwoPaneMinWidth = 840;
    private const double ListWidth = 360;

    private AnalyzeTool? _tool;
    private bool _wide = true;
    private bool _connected;
    private int? _activeId;
    private JsonObject _messages = new();
    private JsonObject _vibration = new();
    private JsonObject _inspector = new();
    private (AnalyzeTool? Tool, bool Gated, int? Reload)? _shown;
    private readonly IReadOnlyList<ToolRow> _rows;
    private readonly DispatcherTimer _noticeTimer = new() { Interval = TimeSpan.FromSeconds(3) };

    public AnalyzePage()
    {
        InitializeComponent();
        _rows = AnalyzeTool.All.Select(Row).ToList();
        ToolList.Fill(_rows
            .GroupBy(row => row.Tool.Section)
            .SelectMany(section => section.Select(row => (UIElement)row.Button).Prepend(Ui.SectionHeader(section.Key))));
        _noticeTimer.Tick += (_, _) =>
        {
            _noticeTimer.Stop();
            Notice.IsOpen = false;
        };
        Ui.Styled(BackButton, Look.Subtle, () => Select(null));
        SizeChanged += (_, size) =>
        {
            _wide = size.NewSize.Width >= TwoPaneMinWidth;
            Arrange();
        };
        Loaded += (_, _) => Attach();
        Unloaded += (_, _) => Detach();
    }

    private void Attach()
    {
        NavigationLock.Refused += OnRefused;
        Watch(FlyStateClient, NoVehicleState.FlyStatePath, view =>
        {
            _connected = !NoVehicleState.IsNoVehicle(view);
            Render();
        });
        Watch(VehiclesClient, VehiclesPath, view =>
        {
            _activeId = ActiveVehicleId(view);
            ShowTool();
        });
        Watch(MessagesClient, MessagesTool.MessagesPath, view =>
        {
            _messages = view;
            RenderList();
        });
        Watch(VibrationClient, VibrationTool.VibrationPath, view =>
        {
            _vibration = view;
            RenderList();
        });
        Watch(InspectorClient, InspectorTool.InspectorPath, view =>
        {
            _inspector = view;
            RenderList();
        });
    }

    private void Detach()
    {
        NavigationLock.Refused -= OnRefused;
        BridgeWatch.Stop(FlyStateClient);
        BridgeWatch.Stop(VehiclesClient);
        BridgeWatch.Stop(MessagesClient);
        BridgeWatch.Stop(VibrationClient);
        BridgeWatch.Stop(InspectorClient);
    }

    private static void Watch(string client, string path, Action<JsonObject> apply)
    {
        BridgeWatch.Watch(client, [path], apply);
        apply(Bridge.Group(path));
    }

    internal static int? ActiveVehicleId(JsonObject vehicles) =>
        Ui.Int(vehicles["activeId"], -1) is var id && id > 0 ? id : (int?)null;

    private void OnRefused(object? sender, string reason)
    {
        Notice.Message = reason;
        Notice.IsOpen = true;
        _noticeTimer.Stop();
        _noticeTimer.Start();
    }

    private void Render()
    {
        RenderList();
        ShowTool();
    }

    private void Select(AnalyzeTool? tool)
    {
        if (tool == _tool || (_tool != null && !NavigationLock.Allows())) return;
        _tool = tool;
        Render();
        Arrange();
    }

    private void Arrange()
    {
        var list = _wide || _tool == null;
        var detail = _wide || _tool != null;
        ListPane.Visibility = Ui.Shown(list);
        DetailPane.Visibility = Ui.Shown(detail);
        ListColumn.Width = _wide ? new GridLength(ListWidth) : list ? new GridLength(1, GridUnitType.Star) : new GridLength(0);
        DetailColumn.Width = detail ? new GridLength(1, GridUnitType.Star) : new GridLength(0);
        BackButton.Visibility = Ui.Shown(!_wide);
        TitleBar.Padding = new Thickness(_wide ? 24 : 8, 0, 24, 0);
    }

    private void ShowTool()
    {
        var gated = _tool is { RequiresVehicle: true } && !_connected;
        var key = (_tool, gated, _tool is { RequiresVehicle: true } ? _activeId : null);
        if (_shown == key) return;
        _shown = key;
        TitleBar.Visibility = Ui.Shown(_tool != null);
        ToolTitle.Text = _tool?.Label ?? "";
        ToolHost.Content = _tool switch
        {
            null => Ui.EmptyState("\uE9D9", "Analyze", "Choose a tool on the left."),
            _ when gated => Ui.EmptyState(_tool.Glyph, RequiresVehicle, ""),
            _ => Create(_tool),
        };
    }

    private static UIElement Create(AnalyzeTool tool) => tool.Key switch
    {
        "LogDownload" => new LogDownloadTool(),
        "GeoTag" => new GeoTagTool(),
        "Vibration" => new VibrationTool(),
        "Inspector" => new InspectorTool(),
        "Console" => new ConsoleTool(),
        "Messages" => new MessagesTool(),
        _ => new FirmwareTool(),
    };

    private void RenderList()
    {
        var unread = MessagesTool.Unread(_messages);
        var messages = MessagesTool.Parse(_messages);
        var reading = VibrationTool.Reading(_vibration);
        var level = reading is null ? null : VibrationTool.Worst(reading);
        var rate = InspectorTool.RateText(_inspector);
        var caveat = _connected ? VibrationTool.Caveat(_vibration) : null;
        _rows.ToList().ForEach(row => Paint(row, Status(row.Tool, unread, level), Subtitle(row.Tool, messages, reading, rate, caveat)));
    }

    private static (string Status, RowState State) Status(AnalyzeTool tool, int unread, string? level) => tool switch
    {
        _ when tool == AnalyzeTool.Messages && unread > 0 => ($"{unread}", RowState.Attention),
        _ when tool == AnalyzeTool.Vibration && level is "danger" or "warning" => (VibrationTool.SeverityLabel(level), RowState.Attention),
        _ when tool == AnalyzeTool.Vibration && level == "normal" => (VibrationTool.SeverityLabel(level), RowState.Done),
        _ => ("", RowState.Neutral),
    };

    private static string Subtitle(AnalyzeTool tool, IReadOnlyList<VehicleMessage> messages, VibrationReading? reading, string? rate, string? caveat)
    {
        var line = tool switch
        {
            _ when tool == AnalyzeTool.Inspector => rate ?? tool.Description,
            _ when tool == AnalyzeTool.Messages && messages.Count > 0 => MessagesTool.Summary(messages),
            _ when tool == AnalyzeTool.Vibration => (reading is null ? null : VibrationTool.Glance(reading)) ?? tool.Description,
            _ => tool.Description,
        };
        var note = tool == AnalyzeTool.Vibration ? caveat : null;
        return string.Join("\n", new[] { line, note }.OfType<string>());
    }

    private void Paint(ToolRow row, (string Text, RowState State) status, string subtitle)
    {
        var selected = row.Tool == _tool;
        var attention = status.State == RowState.Attention;
        row.Marker.Visibility = Ui.Shown(selected);
        row.Badge.Background = Ui.Token(attention ? "CautionBgBrush" : "LayerBrush");
        row.Glyph.Foreground = Ui.Token(attention ? "CautionBrush" : "TextSecondaryBrush");
        row.Title.FontWeight = selected ? FontWeights.SemiBold : FontWeights.Normal;
        row.Subtitle.Text = subtitle;
        row.Status.Text = status.Text;
        row.Status.Foreground = Ui.Token(status.State switch
        {
            RowState.Attention => "CautionBrush",
            RowState.Done => "SuccessBrush",
            _ => "TextTertiaryBrush",
        });
        if (selected) row.Button.Background = Ui.Token("SubtleSelectedBrush");
        else row.Button.ClearValue(Control.BackgroundProperty);
    }

    private ToolRow Row(AnalyzeTool tool)
    {
        var glyph = Ui.Glyph(tool.Glyph, 20, "TextSecondaryBrush").With(g => g.HorizontalAlignment = HorizontalAlignment.Center);
        var badge = Ui.Circle(40, "LayerBrush", glyph);
        var title = Ui.Text(tool.Label);
        var subtitle = Ui.Caption(tool.Description);
        var status = Ui.Caption("", "TextTertiaryBrush");
        var marker = new Border
        {
            Width = 3,
            Height = 16,
            Margin = new Thickness(-12, 0, 0, 0),
            CornerRadius = new CornerRadius(1.5),
            HorizontalAlignment = HorizontalAlignment.Left,
            VerticalAlignment = VerticalAlignment.Center,
            Background = Ui.Token("AccentBrush"),
            Visibility = Visibility.Collapsed,
        };
        var button = Ui.Styled(new Button
        {
            Content = new Grid
            {
                Children =
                {
                    marker,
                    new Grid
                    {
                        ColumnSpacing = 16,
                        ColumnDefinitions =
                        {
                            new ColumnDefinition { Width = GridLength.Auto },
                            new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) },
                            new ColumnDefinition { Width = GridLength.Auto },
                        },
                        Children =
                        {
                            badge,
                            new StackPanel
                            {
                                Spacing = 2,
                                VerticalAlignment = VerticalAlignment.Center,
                                Children = { title, subtitle },
                            }.With(panel => Grid.SetColumn(panel, 1)),
                            status.With(text => Grid.SetColumn(text, 2)),
                        },
                    },
                },
            },
            HorizontalAlignment = HorizontalAlignment.Stretch,
            HorizontalContentAlignment = HorizontalAlignment.Stretch,
            MinHeight = 64,
            Padding = new Thickness(16, 10, 16, 10),
            Margin = new Thickness(8, 1, 8, 1),
            CornerRadius = (CornerRadius)Application.Current.Resources["RadiusControl"],
        }.Named(tool.Label), Look.Subtle, () => Select(tool));
        return new ToolRow(tool, button, marker, badge, glyph, title, subtitle, status);
    }
}
