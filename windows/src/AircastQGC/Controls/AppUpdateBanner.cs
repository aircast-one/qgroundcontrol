using System;
using System.Collections.Generic;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Pages.Analyze;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

public sealed class AppUpdateBanner : UserControl
{
    private readonly string _client = $"appUpdateBanner.{Guid.NewGuid():N}";
    private readonly FontIcon _icon = Ui.Glyph("", 20, "AccentBrush");
    private readonly TextBlock _title = Ui.Text("", semibold: true);
    private readonly TextBlock _detail = Ui.Caption("", "CriticalBrush").With(text => text.Visibility = Visibility.Collapsed);
    private readonly ItemsControl _actions = Ui.Line([], 8);
    private bool _installWhenReady;

    public AppUpdateBanner()
    {
        MaxWidth = 560;
        Visibility = Visibility.Collapsed;
        Content = new Border
        {
            Background = Ui.Token("CardBrush"),
            BorderBrush = Ui.Token("CardStrokeBrush"),
            BorderThickness = new Thickness(1),
            CornerRadius = (CornerRadius)Application.Current.Resources["RadiusOverlay"],
            Padding = new Thickness(12, 8, 12, 8),
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
                    _icon,
                    new StackPanel { Spacing = 2, VerticalAlignment = VerticalAlignment.Center, Children = { _title, _detail } }.With(text => Grid.SetColumn(text, 1)),
                    _actions.With(actions =>
                    {
                        Grid.SetColumn(actions, 2);
                        actions.VerticalAlignment = VerticalAlignment.Center;
                    }),
                },
            },
        }.Named("Update available");
        Loaded += (_, _) =>
        {
            if (!CoreHost.Started) return;
            BridgeWatch.Watch(_client, [AppUpdates.View], Apply);
            Refresh();
        };
        Unloaded += (_, _) => BridgeWatch.Stop(_client);
    }

    private void Refresh() => Ui.Off(this, () => Bridge.Get(AppUpdates.View), Apply);

    private void Apply(JsonObject view)
    {
        var update = AppUpdate.Parse(view);
        _installWhenReady &= update.Phase is AppUpdate.Downloading or AppUpdate.Ready;
        if (_installWhenReady && update.Phase == AppUpdate.Ready && update.CanInstall)
        {
            _installWhenReady = false;
            _ = AppUpdates.Install(XamlRoot, update);
        }
        var offered = update.Offered && !update.PutOff && (AppUpdates.Installed || update.Phase == AppUpdate.Available);
        Visibility = Ui.Shown(update.CanInstall && (update.JustUpdated || offered));
        if (update.JustUpdated) Welcome(update);
        else if (offered) Offer(update);
    }

    private void Welcome(AppUpdate update)
    {
        _icon.Glyph = "";
        _icon.Foreground = Ui.Token("SuccessBrush");
        _title.Text = update.Updated;
        _actions.Fill([
            Ui.Button("What's new", () => Open(update.WhatsNew), Look.Subtle),
            Ui.Button("Dismiss", () => Run("appUpdate.dismissUpdated"), Look.Subtle),
        ]);
    }

    private void Offer(AppUpdate update)
    {
        _icon.Glyph = "";
        _icon.Foreground = Ui.Token("AccentBrush");
        _title.Text = update.Title;
        _actions.Fill(Actions(update));
    }

    private IEnumerable<UIElement?> Actions(AppUpdate update) => update.Phase switch
    {
        AppUpdate.Downloading => [Ui.Button("Cancel", () => Run("appUpdate.cancel"))],
        AppUpdate.Ready => [Ui.Button("Install", () => _ = AppUpdates.Install(XamlRoot, update), Look.Accent), Later()],
        _ when AppUpdates.Installed => [Ui.Button(update.UpdateLabel, () => Update(update), Look.Accent), Later()],
        _ => [Ui.Button("Download", () => Open(update.ReleaseUrl), Look.Accent), Later()],
    };

    private Button Later() => Ui.Button("Later", () => Run("appUpdate.later"), Look.Subtle);

    private async void Update(AppUpdate update)
    {
        if (!await AppUpdates.MayDownload(XamlRoot, update)) return;
        var refusal = await AppUpdates.Invoke("appUpdate.download");
        Say(refusal);
        _installWhenReady = refusal is null;
        Refresh();
    }

    private async void Run(string path) => Say(await AppUpdates.Invoke(path));

    private async void Open(string url) => Say(await AppUpdates.OpenRelease(url));

    private void Say(string? refusal)
    {
        _detail.Text = refusal ?? "";
        _detail.Visibility = Ui.Shown(refusal is not null);
    }
}
