using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

public sealed partial class PreflightChecklist : UserControl
{
    public const string PreflightPath = "view.preflight";
    private const string PreflightClient = "preflightChecklist.preflight";
    private const string FlyStateClient = "preflightChecklist.flyState";

    private IReadOnlyList<PreflightGroup> _groups = [];
    private ImmutableHashSet<string> _ticked = ImmutableHashSet<string>.Empty;
    private string _signature = "";

    public event EventHandler? StateChanged;
    public event EventHandler? Closed;

    public string Airframe { get; private set; } = "Generic";
    public bool Offered { get; private set; } = true;
    public bool HasVehicle { get; private set; }
    public bool Ready { get; private set; }
    public string Progress { get; private set; } = "";
    public bool IsOpen => Visibility == Visibility.Visible;

    public PreflightChecklist()
    {
        InitializeComponent();
        PreflightLook.Standard(ResetButton);
        PreflightLook.Accent(DoneButton);
        Loaded += (_, _) => Attach();
        Unloaded += (_, _) => Detach();
    }

    public void Show() => Visibility = Visibility.Visible;

    public void Hide()
    {
        Visibility = Visibility.Collapsed;
        Closed?.Invoke(this, EventArgs.Empty);
    }

    public void Reset()
    {
        _ticked = ImmutableHashSet<string>.Empty;
        Render();
    }

    private void Attach()
    {
        BridgeWatch.Watch(FlyStateClient, new[] { NoVehicleState.FlyStatePath }, ApplyFlyState);
        BridgeWatch.Watch(PreflightClient, new[] { PreflightPath }, ApplyPreflight);
        ApplyFlyState(Bridge.Group(NoVehicleState.FlyStatePath));
        ApplyPreflight(Bridge.Group(PreflightPath));
    }

    private static void Detach()
    {
        BridgeWatch.Stop(FlyStateClient);
        BridgeWatch.Stop(PreflightClient);
    }

    private void ApplyFlyState(JsonObject flyState)
    {
        var connected = !NoVehicleState.IsNoVehicle(flyState);
        if (connected == HasVehicle) return;
        _ticked = HasVehicle ? ImmutableHashSet<string>.Empty : _ticked;
        HasVehicle = connected;
        Render();
    }

    private void ApplyPreflight(JsonObject view)
    {
        var signature = view.ToJsonString();
        if (signature == _signature) return;
        _signature = signature;
        Airframe = PreflightJson.Text(view["airframe"]) is { Length: > 0 } airframe ? airframe : "Generic";
        Offered = PreflightJson.Flag(view["offered"], fallback: true);
        _groups = PreflightGroup.Parse(view["groups"]);
        Render();
    }

    private void Toggle(string name)
    {
        var check = _groups.SelectMany(g => g.Checks).FirstOrDefault(c => c.Name == name);
        if (check is not { Tickable: true }) return;
        _ticked = _ticked.Contains(name) ? _ticked.Remove(name) : _ticked.Add(name);
        Render();
    }

    private void Render()
    {
        var checks = _groups.SelectMany(g => g.Checks).ToList();
        var settled = checks.Count(c => c.Met(_ticked));
        Progress = $"{settled} of {checks.Count} checked";
        Ready = checks.Count > 0 && settled == checks.Count;
        Subtitle.Text = $"{Airframe} · {Progress}";
        ReadyPill.Visibility = Ready ? Visibility.Visible : Visibility.Collapsed;
        EmptyText.Visibility = checks.Count == 0 ? Visibility.Visible : Visibility.Collapsed;
        GroupList.ItemsSource = _groups.Select(GroupView).ToList();
        StateChanged?.Invoke(this, EventArgs.Empty);
    }

    private UIElement GroupView(PreflightGroup group) => new StackPanel
    {
        Spacing = 4,
        Children =
        {
            new TextBlock
            {
                Text = group.Name,
                Style = (Style)Application.Current.Resources["StatLabel"],
                TextWrapping = TextWrapping.Wrap,
            },
            new Border
            {
                Padding = new Thickness(4),
                Background = PreflightLook.Token("LayerBrush"),
                BorderBrush = PreflightLook.Token("CardStrokeBrush"),
                BorderThickness = new Thickness(1),
                CornerRadius = new CornerRadius(4),
                Child = new ItemsControl
                {
                    IsTabStop = false,
                    ItemsSource = group.Checks.Select(Row).ToList(),
                },
            },
        },
    };

    private UIElement Row(PreflightCheck check)
    {
        var met = check.Met(_ticked);
        var text = new StackPanel
        {
            Spacing = 2,
            Children =
            {
                Label(check.Name, "FontSizeBody", "TextPrimaryBrush"),
                Label(check.Reason, "FontSizeCaption", "TextSecondaryBrush"),
            },
        };
        Grid.SetColumn(text, 1);
        var row = new Button
        {
            HorizontalAlignment = HorizontalAlignment.Stretch,
            HorizontalContentAlignment = HorizontalAlignment.Stretch,
            Padding = new Thickness(8),
            IsEnabled = check.Tickable,
            Content = new Grid
            {
                ColumnSpacing = 12,
                ColumnDefinitions =
                {
                    new ColumnDefinition { Width = GridLength.Auto },
                    new ColumnDefinition(),
                },
                Children =
                {
                    new FontIcon
                    {
                        Glyph = Glyph(check, met),
                        FontSize = 16,
                        Foreground = PreflightLook.Token(Tone(check, met)),
                        VerticalAlignment = VerticalAlignment.Top,
                        Margin = new Thickness(0, 2, 0, 0),
                    },
                    text,
                },
            },
        };
        PreflightLook.Subtle(row);
        ToolTipService.SetToolTip(row, check.Hint);
        AutomationProperties.SetName(row, $"{check.Name}, {Spoken(check, met)}");
        row.Click += (_, _) => Toggle(check.Name);
        return row;
    }

    private static TextBlock Label(string text, string size, string brush) => new()
    {
        Text = text,
        Visibility = text.Length == 0 ? Visibility.Collapsed : Visibility.Visible,
        FontFamily = PreflightLook.Font,
        FontSize = PreflightLook.Size(size),
        Foreground = PreflightLook.Token(brush),
        TextWrapping = TextWrapping.Wrap,
    };

    private static string Glyph(PreflightCheck check, bool met) =>
        met ? "" : check.Blocks ? "" : check.Warns ? "" : "";

    private static string Tone(PreflightCheck check, bool met) =>
        check.Blocks ? "CriticalBrush" : met ? "SuccessBrush" : check.Warns ? "CautionBrush" : "TextSecondaryBrush";

    private static string Spoken(PreflightCheck check, bool met) =>
        met ? "checked" : check.Blocks ? $"blocked: {check.Hint}" : "not checked";

    private void OnReset(object sender, RoutedEventArgs e) => Reset();

    private void OnDone(object sender, RoutedEventArgs e) => Hide();
}
