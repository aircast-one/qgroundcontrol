using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Pages.Analyze;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Shapes;
using Windows.Foundation;

namespace AircastQGC.Controls;

internal static class FleetLook
{
    private static readonly (double X, double Y)[] ArrowOutline = [(0.5, 0.14), (0.75, 0.8), (0.5, 0.67), (0.25, 0.8)];

    internal static Polygon Arrow(double size, string fill, string stroke, double? heading)
    {
        var points = new PointCollection();
        ArrowOutline.Select(at => new Point(at.X * size, at.Y * size)).ToList().ForEach(points.Add);
        return new Polygon
        {
            Width = size,
            Height = size,
            Points = points,
            Fill = Ui.Token(fill),
            Stroke = Ui.Token(stroke),
            StrokeThickness = 1.5,
            StrokeLineJoin = PenLineJoin.Round,
            Visibility = Ui.Shown(heading.HasValue),
            RenderTransform = new RotateTransform { Angle = heading ?? 0, CenterX = size / 2, CenterY = size / 2 },
        };
    }

    internal static Style Presenter(double maxWidth) => new(typeof(FlyoutPresenter))
    {
        Setters =
        {
            new Setter(Control.BackgroundProperty, Ui.Token("CardBrush")),
            new Setter(Control.BorderBrushProperty, Ui.Token("CardStrokeBrush")),
            new Setter(Control.BorderThicknessProperty, new Thickness(1)),
            new Setter(Control.CornerRadiusProperty, Application.Current.Resources["RadiusOverlay"]),
            new Setter(Control.PaddingProperty, new Thickness(0)),
            new Setter(FrameworkElement.MaxWidthProperty, maxWidth),
        },
    };
}

public sealed class FleetPanel : UserControl
{
    private const double CompassSize = 28;
    private const double DisarmedOpacity = 0.5;

    private sealed class Row
    {
        internal required Grid View { get; init; }
        internal required CheckBox Pick { get; init; }
        internal required Button Fly { get; init; }
        internal required TextBlock Name { get; init; }
        internal required TextBlock Line { get; init; }
        internal required TextBlock Telemetry { get; init; }
        internal required Button Mode { get; init; }
        internal required Grid Compass { get; init; }
        internal required Polygon Heading { get; init; }
        internal required FontIcon Flying { get; init; }
        internal VehicleChoice? Choice { get; set; }
    }

    private readonly string _client = $"fleetPanel.{Guid.NewGuid():N}";
    private readonly StackPanel _rows = new();
    private readonly TextBlock _selectedHeading = Ui.Text("", "FontSizeBody", semibold: true);
    private readonly Button _selectAll;
    private readonly Button _deselectAll;
    private readonly ItemsControl _actions = Ui.Column([], 2);
    private readonly StackPanel _controls;
    private readonly TextBlock _refusal = Ui.Caption("", "CriticalBrush");
    private ImmutableDictionary<int, Row> _byId = ImmutableDictionary<int, Row>.Empty;
    private JsonObject _view = new();
    private bool _painting;
    private string _actionKey = "";
    private string _orderKey = "";

    public FleetPanel()
    {
        _selectAll = Ui.Button("Select all", () => Refuse(FleetBridge.SelectAll(Choices)), Look.Subtle);
        _deselectAll = Ui.Button("Deselect all", () => Refuse(FleetBridge.DeselectAll()), Look.Subtle);
        _controls = new StackPanel
        {
            Spacing = 4,
            Padding = new Thickness(16, 8, 16, 0),
            Children =
            {
                _selectedHeading,
                new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8, Children = { _selectAll, _deselectAll } },
                Ui.Text("Multi vehicle actions", "FontSizeBody", semibold: true).With(text => text.Margin = new Thickness(0, 8, 0, 0)),
                _actions,
            },
        };
        _refusal.Margin = new Thickness(16, 8, 16, 0);
        Content = new StackPanel { Children = { _rows, _controls, _refusal } };

        Loaded += (_, _) => Start();
        Unloaded += (_, _) => Stop();
    }

    public event EventHandler<GuidedAsk>? ConfirmRequested;

    public event EventHandler? Switched;

    public event EventHandler? Changed;

    internal VehicleChoices Choices { get; private set; } = VehicleChoices.None;

    public bool PanelEnabled { get; private set; } = true;

    public int Count => Choices.Choices.Count;

    internal void Stop()
    {
        BridgeWatch.Stop($"{_client}.vehicles");
        BridgeWatch.Stop($"{_client}.panel");
    }

    internal void Start()
    {
        if (!CoreHost.Started) return;
        BridgeWatch.Watch($"{_client}.vehicles", new[] { VehicleChoices.ViewPath }, Apply);
        BridgeWatch.Watch($"{_client}.panel", new[] { FleetBridge.PanelValue }, setting =>
        {
            PanelEnabled = FleetBridge.PanelEnabled(setting);
            Render();
        });
        PanelEnabled = FleetBridge.PanelEnabled(Bridge.Get(FleetBridge.PanelValue));
        Apply(Bridge.Get(VehicleChoices.ViewPath));
    }

    private void Apply(JsonObject view)
    {
        _view = view;
        Choices = VehicleChoices.Parse(view);
        Render();
    }

    private void Render()
    {
        _painting = true;
        var choices = Choices;
        var distinguishes = choices.LinkDistinguishes;
        var listed = choices.Choices.Select(choice => choice.Id).ToHashSet();
        _byId = _byId.RemoveRange(_byId.Keys.Where(id => !listed.Contains(id)).ToList())
            .SetItems(choices.Choices.Where(choice => !_byId.ContainsKey(choice.Id)).Select(choice => KeyValuePair.Create(choice.Id, Build())).ToList());
        choices.Choices.ToList().ForEach(choice => Paint(_byId[choice.Id], choice, distinguishes));
        var order = string.Join(",", choices.Choices.Select(choice => choice.Id));
        if (order != _orderKey)
        {
            _orderKey = order;
            _rows.Children.Clear();
            choices.Choices.Select(choice => _byId[choice.Id].View).ToList().ForEach(_rows.Children.Add);
        }

        _controls.Visibility = Ui.Shown(PanelEnabled && choices.Ambiguous);
        _selectedHeading.Text = choices.FleetHeading;
        _selectAll.IsEnabled = choices.CanSelectAll;
        _deselectAll.IsEnabled = choices.CanDeselectAll;
        var actionKey = _view["actions"]?.ToJsonString() ?? "";
        if (actionKey != _actionKey)
        {
            _actionKey = actionKey;
            _actions.Fill(choices.Actions.Select(ActionRow));
        }
        _painting = false;
        Changed?.Invoke(this, EventArgs.Empty);
    }

    private Row Build()
    {
        var name = Ui.Text("", "FontSizeBody", wrap: false);
        var line = Ui.Caption("", "TextSecondaryBrush").With(text => text.TextTrimming = TextTrimming.CharacterEllipsis);
        var telemetry = Ui.Caption("", "TextTertiaryBrush").With(text => text.TextTrimming = TextTrimming.CharacterEllipsis);
        var fly = Ui.Styled(new Button
        {
            HorizontalAlignment = HorizontalAlignment.Stretch,
            HorizontalContentAlignment = HorizontalAlignment.Left,
            Padding = new Thickness(8, 6, 8, 6),
            MinHeight = 56,
            CornerRadius = (CornerRadius)Application.Current.Resources["RadiusControl"],
            Content = new StackPanel { Spacing = 2, Children = { name, line, telemetry } },
        }, Look.Subtle, () => { });
        var pick = new CheckBox { MinWidth = 0, VerticalAlignment = VerticalAlignment.Center };
        var mode = Ui.Button("Mode", () => { }, Look.Subtle);
        var heading = FleetLook.Arrow(CompassSize, "AccentBrush", "AccentOnBrush", null);
        var compass = new Grid
        {
            Width = CompassSize,
            Height = CompassSize,
            VerticalAlignment = VerticalAlignment.Center,
            Children = { new Ellipse { Fill = Ui.Token("LayerBrush"), Stroke = Ui.Token("TextSecondaryBrush"), StrokeThickness = 1 }, heading },
        };
        var flying = Ui.Glyph("\uE73E", 14, "AccentBrush").Named("Flying this one");
        var view = new Grid
        {
            ColumnSpacing = 8,
            Padding = new Thickness(8, 0, 16, 0),
            ColumnDefinitions =
            {
                new ColumnDefinition { Width = GridLength.Auto },
                new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) },
                new ColumnDefinition { Width = GridLength.Auto },
                new ColumnDefinition { Width = GridLength.Auto },
                new ColumnDefinition { Width = GridLength.Auto },
            },
            Children =
            {
                pick,
                fly.With(item => Grid.SetColumn(item, 1)),
                mode.With(item => { Grid.SetColumn(item, 2); item.VerticalAlignment = VerticalAlignment.Center; }),
                compass.With(item => Grid.SetColumn(item, 3)),
                flying.With(item => Grid.SetColumn(item, 4)),
            },
        };
        var row = new Row
        {
            View = view, Pick = pick, Fly = fly, Name = name, Line = line, Telemetry = telemetry,
            Mode = mode, Compass = compass, Heading = heading, Flying = flying,
        };
        fly.Click += (_, _) => Activate(row);
        pick.Checked += (_, _) => Select(row, true);
        pick.Unchecked += (_, _) => Select(row, false);
        var modes = new MenuFlyout { Placement = FlyoutPlacementMode.BottomEdgeAlignedRight };
        modes.Opening += (_, _) => FillModes(modes, row);
        mode.Flyout = modes;
        return row;
    }

    private void Paint(Row row, VehicleChoice choice, bool distinguishes)
    {
        row.Choice = choice;
        row.Name.Text = choice.Name;
        row.Line.Text = choice.Line(distinguishes);
        row.Telemetry.Text = choice.TelemetryLine ?? "";
        row.Telemetry.Visibility = Ui.Shown(choice.TelemetryLine != null);
        row.Fly.Named(choice.Active ? $"{choice.Name}, flying" : $"Fly {choice.Name}");
        row.Pick.Visibility = Ui.Shown(PanelEnabled);
        row.Pick.IsChecked = choice.Selected;
        row.Pick.Named($"Select {choice.Name}");
        row.Mode.Visibility = Ui.Shown(choice.FlightModes.Count > 0 && choice.Index >= 0);
        row.Mode.Named($"Mode for {choice.Name}");
        row.Compass.Opacity = choice.Armed ? 1 : DisarmedOpacity;
        row.Heading.Visibility = Ui.Shown(choice.Heading.HasValue);
        ((RotateTransform)row.Heading.RenderTransform).Angle = choice.Heading ?? 0;
        row.Flying.Visibility = Ui.Shown(choice.Active);
    }

    private void FillModes(MenuFlyout menu, Row row)
    {
        menu.Items.Clear();
        (row.Choice?.FlightModes ?? [])
            .Select(mode => new MenuFlyoutItem { Text = mode, FontFamily = PreflightLook.Font }.With(item =>
                item.Click += (_, _) => { if (row.Choice is { } choice) Refuse(FleetBridge.SetMode(choice, mode)); }))
            .ToList()
            .ForEach(menu.Items.Add);
    }

    private void Activate(Row row)
    {
        if (row.Choice is not { Active: false } choice) return;
        var refusal = FleetBridge.AskFor(choice.Id);
        Refuse(refusal);
        if (refusal == null) Switched?.Invoke(this, EventArgs.Empty);
    }

    private void Select(Row row, bool selected)
    {
        if (_painting || row.Choice is not { } choice || choice.Selected == selected) return;
        Refuse(FleetBridge.SetSelected(choice.Id, selected));
    }

    private UIElement ActionRow(FleetAction action)
    {
        var button = Ui.Styled(new Button
        {
            IsEnabled = action.Ready,
            HorizontalAlignment = HorizontalAlignment.Stretch,
            HorizontalContentAlignment = HorizontalAlignment.Left,
            Padding = new Thickness(8, 6, 8, 6),
            CornerRadius = (CornerRadius)Application.Current.Resources["RadiusControl"],
            Content = new StackPanel
            {
                Spacing = 2,
                Children =
                {
                    Ui.Text(action.Title, "FontSizeBody", action.Ready ? "TextPrimaryBrush" : "TextSecondaryBrush"),
                    Ui.Caption(action.Line, "TextTertiaryBrush"),
                },
            },
        }, Look.Subtle, () => Ask(action));
        return button.Named(action.ConfirmTitle);
    }

    private void Ask(FleetAction action)
    {
        var confirmed = Choices.SelectedIds;
        ConfirmRequested?.Invoke(this, new GuidedAsk(action.ConfirmTitle, action.Prompt, () =>
            FleetBridge.Command(action.Id, confirmed) ? null : $"{action.Title} did not reach every selected aircraft.")
        {
            Destructive = action.Destructive,
            WatchPath = VehicleChoices.ViewPath,
            StillOffered = view => VehicleChoices.Parse(view).Actions.Any(offered => offered.Id == action.Id && offered.Ready),
        });
    }

    internal void Refuse(string? refusal)
    {
        _refusal.Text = refusal ?? "";
        _refusal.Visibility = Ui.Shown(refusal != null);
    }
}

public sealed class FleetCard : UserControl
{
    private const double CardWidth = 440;
    private const double CardHeight = 320;

    private readonly FleetPanel _panel = new();
    private bool _suppressed;

    public FleetCard()
    {
        Visibility = Visibility.Collapsed;
        MaxWidth = CardWidth;
        Content = Ui.Card(new ScrollViewer { MaxHeight = CardHeight, Content = _panel }).With(card => card.Padding = new Thickness(0, 8, 0, 8));
        _panel.Changed += (_, _) => Refresh();
        _panel.ConfirmRequested += (_, ask) => ConfirmRequested?.Invoke(this, ask);
        Loaded += (_, _) => _panel.Start();
        Unloaded += (_, _) => _panel.Stop();
    }

    public event EventHandler<GuidedAsk>? ConfirmRequested;

    public bool Suppressed
    {
        get => _suppressed;
        set
        {
            _suppressed = value;
            Refresh();
        }
    }

    private void Refresh() => Visibility = Ui.Shown(!_suppressed && _panel.Count >= 2 && _panel.PanelEnabled);
}
