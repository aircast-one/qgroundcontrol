using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;
using AircastQGC.Core;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;

namespace AircastQGC.Pages.Analyze;

internal sealed record InspectorMessage(int Index, int Id, string Name, string RateText, long Count, string Path, int CompId,
    string Title, string TargetRateTitle, bool FieldSelected, bool Selected);

internal sealed record InspectorField(string Name, string Type, string Value);

internal sealed record InspectorChoice(int Id, string Title);

internal sealed class InspectorTool : UserControl
{
    internal const string InspectorPath = "view.inspector";
    private readonly string _client = $"analyzeTool.inspector.{System.Guid.NewGuid():N}";
    private const double PlotColumn = 48;

    private readonly TextBlock _notice = Ui.Centered(Ui.Text("", "FontSizeBodyLarge")).With(text => text.Margin = new Thickness(24));
    private readonly Grid _listPane;
    private readonly Grid _detailPane;
    private readonly TextBlock _system = Ui.Caption("", "TextTertiaryBrush");
    private readonly ItemsControl _chips = Ui.Line([], 8);
    private readonly ScrollViewer _chipStrip;
    private readonly TextBox _filter;
    private readonly ItemsControl _rows = Ui.Column([]);
    private readonly TextBlock _noMatch = Ui.Centered(Ui.Text("")).With(text => text.Margin = new Thickness(24));
    private readonly TextBlock _openName = Ui.Text("", semibold: true, mono: true, wrap: false);
    private readonly DropDownButton _rate = Ui.Picker("Message rate");
    private readonly ItemsControl _details = Ui.Column([], 2);
    private readonly InspectorChart[] _chartPanels = [new(0), new(1)];
    private readonly UIElement _plotHeader;
    private readonly ItemsControl _fields = Ui.Column([]);
    private readonly TextBlock _waiting = Ui.Centered(Ui.Text("Waiting for this message to arrive again.")).With(text => text.Margin = new Thickness(24));
    private readonly DispatcherTimer _chartPoll = new() { Interval = TimeSpan.FromMilliseconds(200) };

    private JsonObject _view = new();
    private string? _openPath;
    private string _filterText = "";
    private int? _component;
    private int? _activeSystem;
    private bool _ratesBuilt;
    private InspectorCharts? _charts;
    private string _chartedSignature = "";
    private string _chipSignature = "";
    private string _rowSignature = "";
    private string _fieldSignature = "";
    private IReadOnlyDictionary<string, TextBlock> _rateTexts = new Dictionary<string, TextBlock>();
    private IReadOnlyDictionary<string, TextBlock> _valueTexts = new Dictionary<string, TextBlock>();

    public InspectorTool()
    {
        _filter = new TextBox { PlaceholderText = "Filter messages", Margin = new Thickness(24, 8, 24, 8) }.Named("Filter messages");
        _filter.TextChanged += (_, _) =>
        {
            _filterText = _filter.Text;
            Render();
        };
        _chipStrip = new ScrollViewer
        {
            HorizontalScrollBarVisibility = ScrollBarVisibility.Auto,
            VerticalScrollBarVisibility = ScrollBarVisibility.Disabled,
            HorizontalScrollMode = ScrollMode.Enabled,
            VerticalScrollMode = ScrollMode.Disabled,
            Padding = new Thickness(24, 0, 24, 0),
            Content = _chips,
        };
        _listPane = new Grid
        {
            RowDefinitions =
            {
                new RowDefinition { Height = GridLength.Auto },
                new RowDefinition { Height = GridLength.Auto },
                new RowDefinition { Height = GridLength.Auto },
                new RowDefinition { Height = new GridLength(1, GridUnitType.Star) },
            },
            Children =
            {
                _system.With(text => text.Margin = new Thickness(24, 0, 24, 4)),
                _chipStrip.With(strip => Grid.SetRow(strip, 1)),
                _filter.With(box => Grid.SetRow(box, 2)),
                new ScrollViewer { Content = new StackPanel { Children = { _rows, _noMatch } } }.With(scroll => Grid.SetRow(scroll, 3)),
            },
        };

        _plotHeader = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            HorizontalAlignment = HorizontalAlignment.Right,
            Margin = new Thickness(0, 12, 0, 0),
            Children =
            {
                Ui.Centered(Ui.Caption(InspectorCharts.Labels[0], "TextTertiaryBrush")).With(text => text.Width = PlotColumn),
                Ui.Centered(Ui.Caption(InspectorCharts.Labels[1], "TextTertiaryBrush")).With(text => text.Width = PlotColumn),
            },
        };
        var back = Ui.Styled(new Button
        {
            Content = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8, Children = { Ui.Glyph("\uE72B"), _openName } },
            HorizontalAlignment = HorizontalAlignment.Stretch,
            HorizontalContentAlignment = HorizontalAlignment.Left,
            Padding = new Thickness(8),
        }.Named("Back to messages"), Look.Subtle, Close);
        _detailPane = new Grid
        {
            Visibility = Visibility.Collapsed,
            Children =
            {
                new ScrollViewer
                {
                    Content = new StackPanel
                    {
                        Padding = new Thickness(16, 0, 24, 24),
                        Spacing = 4,
                        Children =
                        {
                            back,
                            _rate.With(rate => rate.Margin = new Thickness(8, 0, 0, 4)),
                            _details.With(details => details.Margin = new Thickness(8, 0, 0, 8)),
                            new Border { Height = 1, Background = Ui.Token("DividerBrush") },
                            _chartPanels[0],
                            _chartPanels[1],
                            _plotHeader,
                            _fields,
                            _waiting,
                        },
                    },
                },
            },
        };
        Content = new Grid { Children = { _listPane, _detailPane, _notice } };

        _chartPoll.Tick += (_, _) => PollCharts();
        Loaded += (_, _) =>
        {
            BridgeWatch.Watch(_client, [InspectorPath], Apply);
            Apply(Bridge.Group(InspectorPath));
            if (_openPath != null) _chartPoll.Start();
        };
        Unloaded += (_, _) =>
        {
            BridgeWatch.Stop(_client);
            _chartPoll.Stop();
        };
    }

    internal static string? RateText(JsonObject view) =>
        MapUiJson.Flag(view["available"]) && view["messages"] is JsonArray listed
        && listed.OfType<JsonObject>().Sum(message => MapUiJson.Number(message["rateHz"]) ?? 0) is var total && total > 0
            ? $"{Math.Round(total, MidpointRounding.AwayFromZero)} messages/s"
            : null;

    internal static IReadOnlyList<InspectorMessage> Messages(JsonObject view) =>
        MapUiJson.Objects(view["messages"])
            .Select((message, at) => new InspectorMessage(
                Ui.Int(message["index"], at),
                Ui.Int(message["id"]),
                MapUiJson.Text(message["name"]),
                MapUiJson.Text(message["rateText"]),
                (long)(MapUiJson.Number(message["count"]) ?? 0),
                MapUiJson.Text(message["path"]),
                Ui.Int(message["compId"]),
                MapUiJson.Text(message["title"]) is { Length: > 0 } title ? title : MapUiJson.Text(message["name"]),
                MapUiJson.Text(message["targetRateTitle"]),
                MapUiJson.Flag(message["fieldSelected"]),
                MapUiJson.Flag(message["selected"])))
            .OrderBy(message => message.Name, StringComparer.Ordinal)
            .ToList();

    internal static IReadOnlyList<InspectorField> Fields(JsonObject view, string messagePath) =>
        Messages(view).Any(message => message.Selected && message.Path == messagePath)
            ? MapUiJson.Objects(view["fields"])
                .Select(field => new InspectorField(MapUiJson.Text(field["name"]), MapUiJson.Text(field["type"]), MapUiJson.Text(field["value"])))
                .ToList()
            : [];

    private static IReadOnlyList<InspectorChoice> Choices(JsonObject view, string key) =>
        MapUiJson.Objects(view[key]).Select(choice => new InspectorChoice(Ui.Int(choice["id"]), MapUiJson.Text(choice["title"]))).ToList();

    internal static string SelectedPathFor(string messagePath) =>
        messagePath.Split(".messages.")[0] + ".selected";

    internal static int MessageIndexIn(string messagePath) =>
        int.TryParse(messagePath[(messagePath.LastIndexOf('.') + 1)..], out var index) ? index : -1;

    private void Apply(JsonObject view)
    {
        _view = view;
        var system = MapUiJson.Number(view["systemId"]) is double id ? (int)id : (int?)null;
        _component = system == _activeSystem ? _component : null;
        _activeSystem = system;
        Render();
    }

    private void Render()
    {
        var empty = MapUiJson.Text(_view["emptyText"]).Trim();
        var messages = Messages(_view);
        var open = _openPath is { } path ? messages.FirstOrDefault(message => message.Path == path) : null;
        _notice.Text = empty;
        _notice.Visibility = Ui.Shown(empty.Length > 0);
        _listPane.Visibility = Ui.Shown(empty.Length == 0 && open == null);
        _detailPane.Visibility = Ui.Shown(empty.Length == 0 && open != null);
        if (empty.Length > 0) return;
        if (open != null) RenderDetail(open);
        else RenderList(messages);
    }

    private void RenderList(IReadOnlyList<InspectorMessage> messages)
    {
        var systems = Choices(_view, "systems");
        var components = Choices(_view, "components");
        var systemId = MapUiJson.Flag(_view["available"]) ? Ui.Int(_view["systemId"]) : 0;
        _system.Text = systemId > 0 ? $"System {systemId}" : "";
        _system.Visibility = Ui.Shown(systemId > 0);
        _chipStrip.Visibility = Ui.Shown(systems.Count > 1 || components.Count > 1);
        var chipSignature = string.Join("|", systems.Concat(components).Select(choice => $"{choice.Id}:{choice.Title}")) + $"#{_activeSystem}#{_component}";
        if (chipSignature != _chipSignature)
        {
            _chipSignature = chipSignature;
            _chips.Fill(
            (systems.Count > 1
                ? systems.Select(system => Chip(system.Title, system.Id == _activeSystem, () =>
                    Ui.Off(() => Bridge.Invoke("mavlinkInspector.setActiveSystem", new JsonArray(JsonValue.Create(system.Id))))))
                : [])
            .Concat(components.Count > 1
                ? components.Prepend(new InspectorChoice(-1, "Comp All")).Select(choice => Chip(choice.Title, (_component ?? -1) == choice.Id, () =>
                {
                    _component = choice.Id >= 0 ? choice.Id : null;
                    Render();
                }))
                : []));
        }
        var shown = messages
            .Where(message => message.Name.Contains(_filterText, StringComparison.OrdinalIgnoreCase) && (_component is null || message.CompId == _component))
            .ToList();
        _noMatch.Text = $"No message matches \"{_filterText}\".";
        _noMatch.Visibility = Ui.Shown(shown.Count == 0);
        var rowSignature = string.Join("|", shown.Select(message => $"{message.Path}:{message.Name}:{message.Title}:{message.CompId}:{message.FieldSelected}"));
        if (rowSignature != _rowSignature)
        {
            _rowSignature = rowSignature;
            var rows = shown.Select(message => (message.Path, Rate: Ui.Caption(message.RateText), Message: message)).ToList();
            _rateTexts = rows.GroupBy(row => row.Path).ToDictionary(group => group.Key, group => group.First().Rate);
            _rows.Fill(rows.Select(row => Row(row.Message, row.Rate)));
            return;
        }
        shown.ToList().ForEach(message =>
        {
            if (_rateTexts.TryGetValue(message.Path, out var rate)) rate.Text = message.RateText;
        });
    }

    private static UIElement Chip(string title, bool on, Action pick)
    {
        var chip = new ToggleButton
        {
            Content = title,
            IsChecked = on,
            Height = 32,
            Padding = new Thickness(12, 0, 12, 0),
            CornerRadius = new CornerRadius(16),
            FontFamily = PreflightLook.Font,
        };
        chip.Click += (_, _) =>
        {
            chip.IsChecked = on;
            pick();
        };
        return chip;
    }

    private UIElement Row(InspectorMessage message, TextBlock rate) => Ui.Styled(new Button
    {
        Content = new Grid
        {
            ColumnSpacing = 16,
            ColumnDefinitions =
            {
                new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) },
                new ColumnDefinition { Width = GridLength.Auto },
            },
            Children =
            {
                new StackPanel
                {
                    Children =
                    {
                        Ui.Text((message.Name.Length > 0 ? message.Name : message.Title) + (message.FieldSelected ? " *" : ""), mono: true, wrap: false),
                        Ui.Caption($"comp {message.CompId}", "TextTertiaryBrush"),
                    },
                },
                rate.With(text => Grid.SetColumn(text, 1)),
            },
        },
        HorizontalAlignment = HorizontalAlignment.Stretch,
        HorizontalContentAlignment = HorizontalAlignment.Stretch,
        MinHeight = 56,
        Padding = new Thickness(16, 8, 16, 8),
        Margin = new Thickness(8, 0, 8, 0),
    }.Named(message.Name), Look.Subtle, () => Open(message));

    private void Open(InspectorMessage message)
    {
        _openPath = message.Path;
        _fieldSignature = "";
        Ui.Off(() => Bridge.Set(SelectedPathFor(message.Path), JsonValue.Create(MessageIndexIn(message.Path))));
        _chartPoll.Start();
        Render();
    }

    private void Close()
    {
        _openPath = null;
        _fieldSignature = "";
        _chartPoll.Stop();
        Render();
    }

    private void RenderDetail(InspectorMessage open)
    {
        _openName.Text = open.Name;
        var rates = MapUiJson.Objects(_view["rateChoices"])
            .Select(choice => (Rate: Ui.Int(choice["rate"]), Title: MapUiJson.Text(choice["title"])))
            .Where(choice => !string.IsNullOrWhiteSpace(choice.Title))
            .ToList();
        _rate.Visibility = Ui.Shown(rates.Count > 0);
        _rate.Content = open.TargetRateTitle.Length == 0 ? "Set Rate:" : $"Rate: {open.TargetRateTitle}";
        if (!_ratesBuilt && rates.Count > 0)
        {
            _ratesBuilt = true;
            _rate.Flyout = Ui.Menu(rates.Select(choice => (choice.Title, (Action)(() => RequestRate(choice.Rate)))));
        }
        _details.Fill(new[]
        {
            ("Message", $"{open.Name} ({open.Id})"),
            ("Component", open.CompId.ToString(CultureInfo.InvariantCulture)),
            ("Count", open.Count.ToString(CultureInfo.InvariantCulture)),
            ("Actual Rate", open.RateText),
        }.Select(detail => Detail(detail.Item1, detail.Item2)));
        RenderFields();
    }

    private void RequestRate(int rate)
    {
        if (_openPath is not { } path) return;
        Lane.Run(() =>
        {
            Bridge.Set(SelectedPathFor(path), JsonValue.Create(MessageIndexIn(path)));
            Bridge.Invoke("mavlinkInspector.setMessageInterval", new JsonArray(JsonValue.Create(rate)));
        });
    }

    private static UIElement Detail(string label, string value) => new Grid
    {
        ColumnDefinitions =
        {
            new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) },
            new ColumnDefinition { Width = GridLength.Auto },
        },
        Children =
        {
            Ui.Caption(label),
            Ui.Text(value, "FontSizeCaption", mono: true).With(text => Grid.SetColumn(text, 1)),
        },
    };

    private void RenderFields()
    {
        var rows = _openPath is { } path ? Fields(_view, path) : [];
        var any = rows.Count > 0;
        _waiting.Visibility = Ui.Shown(!any);
        _plotHeader.Visibility = Ui.Shown(any);
        _fields.Visibility = Ui.Shown(any);
        _chartPanels.ToList().ForEach(panel => panel.Visibility = any ? panel.Visibility : Visibility.Collapsed);
        var signature = string.Join("|", rows.Select(field => $"{field.Name}:{field.Type}")) + "#" + _chartedSignature;
        if (signature != _fieldSignature)
        {
            _fieldSignature = signature;
            var built = rows.Select(field => (field, Value: Ui.Text(field.Value, mono: true))).ToList();
            _valueTexts = built.GroupBy(row => row.field.Name).ToDictionary(group => group.Key, group => group.First().Value);
            _fields.Fill(built.Select(row => FieldRow(row.field, row.Value)));
            return;
        }
        rows.ToList().ForEach(field =>
        {
            if (_valueTexts.TryGetValue(field.Name, out var value)) value.Text = field.Value;
        });
    }

    private UIElement FieldRow(InspectorField field, TextBlock value) => new Grid
    {
        Padding = new Thickness(8, 8, 0, 8),
        BorderBrush = Ui.Token("DividerBrush"),
        BorderThickness = new Thickness(0, 0, 0, 1),
        ColumnSpacing = 12,
        ColumnDefinitions =
        {
            new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) },
            new ColumnDefinition { Width = GridLength.Auto },
            new ColumnDefinition { Width = new GridLength(PlotColumn) },
            new ColumnDefinition { Width = new GridLength(PlotColumn) },
        },
        Children =
        {
            new StackPanel
            {
                Children =
                {
                    Ui.Text(field.Name, mono: true, wrap: false),
                    Ui.Caption(field.Type, "TextTertiaryBrush"),
                },
            },
            value.With(text =>
            {
                Grid.SetColumn(text, 1);
                text.MaxWidth = 280;
                text.IsTextSelectionEnabled = true;
            }),
            PlotBox(field, 0).With(box => Grid.SetColumn(box, 2)),
            PlotBox(field, 1).With(box => Grid.SetColumn(box, 3)),
        },
    };

    private CheckBox PlotBox(InspectorField field, int chart)
    {
        var box = new CheckBox
        {
            IsChecked = _charts?.Charted.TryGetValue(field.Name, out var on) == true && on == chart,
            IsEnabled = InspectorCharts.ToggleEnabled(_charts, field.Name, field.Type, chart),
            MinWidth = 0,
            Padding = new Thickness(0),
            HorizontalAlignment = HorizontalAlignment.Center,
        }.Named($"{field.Name} on {InspectorCharts.Labels[chart]}");
        box.Click += (_, _) => InspectorCharts.Toggle(chart, field.Name, box.IsChecked == true);
        return box;
    }

    private void PollCharts() =>
        Ui.Off(this, () => InspectorCharts.Parse(Bridge.Get(InspectorCharts.ChartsPath)), charts =>
        {
            if (_openPath == null || charts == null) return;
            _charts = charts;
            var fieldsShown = _fields.Visibility == Visibility.Visible;
            _chartPanels.ToList().ForEach(panel =>
            {
                panel.Update(charts);
                panel.Visibility = fieldsShown ? panel.Visibility : Visibility.Collapsed;
            });
            var signature = string.Join(",", charts.Charted.Select(entry => $"{entry.Key}:{entry.Value}"))
                + "|" + string.Join(",", charts.Charts.Select(chart => chart.Room));
            if (signature == _chartedSignature) return;
            _chartedSignature = signature;
            RenderFields();
        });
}
