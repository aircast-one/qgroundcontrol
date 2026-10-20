using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Controls;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;

namespace AircastQGC.Pages.Plan;

internal sealed class PlanDialogs(FrameworkElement owner)
{
    private const double ListHeight = 360;
    private const int VehicleSystem = 3;
    private bool _showing;

    public bool Showing => _showing;

    private ContentDialog Dialog(string title, object content, string close) => new()
    {
        Title = title,
        Content = content,
        CloseButtonText = close,
        Style = owner.Resources.TryGetValue("PlanDialog", out var style) ? (Style)style : null,
        RequestedTheme = ElementTheme.Dark,
        DefaultButton = ContentDialogButton.Primary,
    };

    private async Task<ContentDialogResult> Show(ContentDialog dialog)
    {
        if (_showing || owner.XamlRoot is null) return ContentDialogResult.None;
        _showing = true;
        try
        {
            dialog.XamlRoot = owner.XamlRoot;
            return await dialog.ShowAsync();
        }
        catch (COMException)
        {
            return ContentDialogResult.None;
        }
        finally
        {
            _showing = false;
        }
    }

    public async Task<bool> Confirm(string title, string body, string confirm, string cancel = "Cancel", bool destructive = false)
    {
        var dialog = Dialog(title, PlanFacts.Text(body, "TextSecondaryBrush"), cancel);
        dialog.PrimaryButtonText = confirm;
        if (destructive)
        {
            dialog.PrimaryButtonStyle = Danger();
            dialog.DefaultButton = ContentDialogButton.Close;
        }
        return await Show(dialog) == ContentDialogResult.Primary;
    }

    public async Task<bool?> Choose(string title, string body, string primary, string secondary, bool dismissable = true)
    {
        var dialog = Dialog(title, PlanFacts.Text(body, "TextSecondaryBrush"), dismissable ? "Cancel" : string.Empty);
        dialog.PrimaryButtonText = primary;
        dialog.SecondaryButtonText = secondary;
        if (!dismissable) dialog.Closing += (_, args) => args.Cancel = args.Result == ContentDialogResult.None;
        return await Show(dialog) switch
        {
            ContentDialogResult.Primary => true,
            ContentDialogResult.Secondary => false,
            _ => null,
        };
    }

    public async Task<T?> Pick<T>(string title, string body, IReadOnlyList<(T Value, string Label, bool Enabled)> options) where T : class
    {
        var chosen = default(T);
        var list = new StackPanel { Spacing = 4 };
        var dialog = Dialog(title, new StackPanel { Spacing = 12, MinWidth = 360, Children = { PlanFacts.Text(body, "TextSecondaryBrush"), list } }, "Cancel");
        options.Select(option =>
        {
            var button = new Button { Content = option.Label, IsEnabled = option.Enabled, HorizontalAlignment = HorizontalAlignment.Stretch, HorizontalContentAlignment = HorizontalAlignment.Left };
            button.Click += (_, _) =>
            {
                chosen = option.Value;
                dialog.Hide();
            };
            return button;
        }).ToList().ForEach(list.Children.Add);
        await Show(dialog);
        return chosen;
    }

    public async Task<int?> PickCommand(string? itemCategory)
    {
        var categories = PlanCalls.Categories();
        var chosen = default(int?);
        var commands = new StackPanel { Spacing = 2 };
        var category = new ComboBox { HorizontalAlignment = HorizontalAlignment.Stretch };
        AutomationProperties.SetName(category, "Category");
        categories.ToList().ForEach(name => category.Items.Add(MapUiJson.SentenceCase(name)));
        var dialog = Dialog("Select mission command", new StackPanel
        {
            Spacing = 8,
            MinWidth = 420,
            Children = { category, new ScrollViewer { Height = ListHeight, Content = commands } },
        }, "Cancel");

        void Fill(int at)
        {
            commands.Children.Clear();
            if (at < 0 || at >= categories.Count) return;
            PlanCalls.Commands(categories[at]).Select(command =>
            {
                var name = MapUiJson.SentenceCase(command.Name);
                var row = new StackPanel { Spacing = 2 };
                row.Children.Add(PlanFacts.Text(name));
                if (command.Description.Length > 0) row.Children.Add(PlanFacts.Note(command.Description));
                var entry = new Button
                {
                    Content = row,
                    HorizontalAlignment = HorizontalAlignment.Stretch,
                    HorizontalContentAlignment = HorizontalAlignment.Left,
                    Background = new SolidColorBrush(Microsoft.UI.Colors.Transparent),
                    BorderThickness = new Thickness(0),
                };
                AutomationProperties.SetName(entry, name);
                entry.Click += (_, _) =>
                {
                    chosen = command.Command;
                    dialog.Hide();
                };
                return entry;
            }).ToList().ForEach(commands.Children.Add);
        }

        category.SelectionChanged += (_, _) => Fill(category.SelectedIndex);
        var start = itemCategory != null && categories.Contains(itemCategory) ? categories.ToList().IndexOf(itemCategory) : 0;
        category.SelectedIndex = categories.Count == 0 ? -1 : start;
        if (categories.Count == 0) commands.Children.Add(PlanFacts.Note("The vehicle offers no mission commands."));
        await Show(dialog);
        return chosen;
    }

    public async Task<Geo?> EditPosition(Geo at, bool vehicleOffered, string title = "Edit position", string confirm = "Move", string? vehicleConfirm = null)
    {
        var systems = new[] { "Geographic", "Universal Transverse Mercator", "Military Grid Reference", "Vehicle Position" }
            .Take(vehicleOffered ? 4 : 3)
            .ToList();
        var system = new ComboBox { HorizontalAlignment = HorizontalAlignment.Stretch };
        systems.ForEach(name => system.Items.Add(name));
        var forms = PlanCalls.Read(PlanRules.Invariant($"view.positionForms({at.Latitude:F8},{at.Longitude:F8})"));
        var utm = forms["utm"] as JsonObject;
        var latitude = Field("Latitude", at.Latitude.ToString("F7", CultureInfo.InvariantCulture));
        var longitude = Field("Longitude", at.Longitude.ToString("F7", CultureInfo.InvariantCulture));
        var zone = Field("Zone", MapUiJson.Number(utm?["zone"]) is double z ? ((int)z).ToString(CultureInfo.InvariantCulture) : string.Empty);
        var southern = new CheckBox { Content = "Southern hemisphere", IsChecked = MapUiJson.Flag(utm?["southern"]) };
        var easting = Field("Easting", MapUiJson.Number(utm?["easting"]) is double east ? east.ToString("F2", CultureInfo.InvariantCulture) : string.Empty);
        var northing = Field("Northing", MapUiJson.Number(utm?["northing"]) is double north ? north.ToString("F2", CultureInfo.InvariantCulture) : string.Empty);
        var mgrs = Field("MGRS", MapUiJson.Text(forms["mgrs"]));
        var vehicle = PlanFacts.Note(string.Empty);
        var problem = PlanFacts.Error(string.Empty);
        var groups = new UIElement[][]
        {
            [latitude, longitude],
            [zone, southern, easting, northing],
            [mgrs],
            [vehicle],
        };
        var fields = new StackPanel { Spacing = 8 };
        var dialog = Dialog(title, new StackPanel { Spacing = 12, MinWidth = 360, Children = { system, fields, problem } }, "Cancel");
        dialog.PrimaryButtonText = confirm;

        void Shown(int chosen)
        {
            fields.Children.Clear();
            groups[Math.Clamp(chosen, 0, groups.Length - 1)].ToList().ForEach(fields.Children.Add);
            dialog.PrimaryButtonText = chosen == VehicleSystem && vehicleConfirm != null ? vehicleConfirm : confirm;
            vehicle.Text = PlanCalls.VehiclePosition() is { } position
                ? PlanRules.Invariant($"{position.Latitude:F7}, {position.Longitude:F7}")
                : "The vehicle has no position yet.";
            problem.Text = string.Empty;
        }

        Geo? Typed() => system.SelectedIndex switch
        {
            0 => PlanRules.TypedCoordinate(latitude.Text, 90) is double lat && PlanRules.TypedCoordinate(longitude.Text, 180) is double lon ? new Geo(lat, lon) : null,
            1 => Converted(PlanCalls.Read($"view.utmToGeo({easting.Text.Trim()},{northing.Text.Trim()},{zone.Text.Trim()},{(southern.IsChecked == true ? "true" : "false")})")),
            2 => Converted(PlanCalls.Read($"view.mgrsToGeo({mgrs.Text.Replace(" ", string.Empty)})")),
            _ => PlanCalls.VehiclePosition(),
        };

        var result = default(Geo?);
        system.SelectionChanged += (_, _) => Shown(system.SelectedIndex);
        dialog.PrimaryButtonClick += (_, args) =>
        {
            result = Typed();
            if (result != null) return;
            problem.Text = "That position could not be read.";
            args.Cancel = true;
        };
        system.SelectedIndex = 0;
        return await Show(dialog) == ContentDialogResult.Primary ? result : null;
    }

    public async Task Content(string title, Func<Action, UIElement> build, string close = "Done")
    {
        var dialog = Dialog(title, null!, close);
        dialog.Content = new ScrollViewer { Content = build(dialog.Hide), MaxHeight = 560, MinWidth = 420 };
        await Show(dialog);
    }

    private static Geo? Converted(JsonObject view) =>
        MapUiJson.Flag(view["valid"]) && MapUiJson.Number(view["latitude"]) is double latitude && MapUiJson.Number(view["longitude"]) is double longitude
            ? new Geo(latitude, longitude)
            : null;

    private static TextBox Field(string header, string text) => new() { Header = header, Text = text };

    private static Style Danger()
    {
        var style = new Style(typeof(Button)) { BasedOn = (Style)Application.Current.Resources["DefaultButtonStyle"] };
        style.Setters.Add(new Setter(Control.ForegroundProperty, (Brush)Application.Current.Resources["CriticalBrush"]));
        return style;
    }
}
