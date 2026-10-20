using System;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Pages.Analyze;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

internal sealed record TerrainLoad(long Loaded, long Pending)
{
    internal bool Waiting => Pending > 0;

    internal static TerrainLoad Parse(JsonObject view) =>
        new((long)(MapUiJson.Number(view["loaded"]) ?? 0), (long)(MapUiJson.Number(view["pending"]) ?? 0));
}

public sealed class TerrainDataWarning : UserControl
{
    private const string TerrainView = "view.terrainDownload";
    internal const string StalledTitle = "Terrain data isn't loading";
    internal const string StalledDetail = "Terrain following may not work. It needs internet and terrain coverage for this area.";

    private readonly string _client = $"fly.terrainDownload.{Guid.NewGuid():N}";
    private readonly DispatcherTimer _stall = new() { Interval = TimeSpan.FromSeconds(30) };
    private (bool Waiting, long Loaded)? _key;
    private bool _stalled;
    private bool _dismissed;

    public TerrainDataWarning()
    {
        MaxWidth = 420;
        Visibility = Visibility.Collapsed;
        Content = new Border
        {
            Background = Ui.Token("CautionBgBrush"),
            CornerRadius = (CornerRadius)Application.Current.Resources["RadiusOverlay"],
            Padding = new Thickness(12, 8, 4, 8),
            Child = new Grid
            {
                ColumnSpacing = 12,
                ColumnDefinitions =
                {
                    new ColumnDefinition { Width = GridLength.Auto },
                    new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) },
                    new ColumnDefinition { Width = GridLength.Auto },
                },
                Children =
                {
                    Ui.Glyph("", 20, "CautionBrush"),
                    new StackPanel
                    {
                        Children = { Ui.Text(StalledTitle, semibold: true), Ui.Caption(StalledDetail) },
                    }.With(text => Grid.SetColumn(text, 1)),
                    Ui.IconButton("", "Dismiss", Dismiss).With(button => Grid.SetColumn(button, 2)),
                },
            },
        };
        _stall.Tick += (_, _) =>
        {
            _stall.Stop();
            _stalled = true;
            Render();
        };
        Loaded += (_, _) =>
        {
            BridgeWatch.Watch(_client, [TerrainView], Apply);
            Ui.Off(this, () => Bridge.Get(TerrainView), Apply);
        };
        Unloaded += (_, _) =>
        {
            BridgeWatch.Stop(_client);
            _stall.Stop();
        };
    }

    private void Apply(JsonObject view)
    {
        var load = TerrainLoad.Parse(view);
        if (_key == (load.Waiting, load.Loaded)) return;
        _key = (load.Waiting, load.Loaded);
        _stall.Stop();
        _stalled = false;
        _dismissed = _dismissed && load.Waiting;
        if (load.Waiting) _stall.Start();
        Render();
    }

    private void Dismiss()
    {
        _dismissed = true;
        Render();
    }

    private void Render() => Visibility = Ui.Shown(_stalled && !_dismissed);
}
