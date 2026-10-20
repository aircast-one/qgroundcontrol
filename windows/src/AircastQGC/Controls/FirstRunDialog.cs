using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Forms;
using AircastQGC.Pages.Settings;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

internal sealed class FirstRunDialog
{
    private const string ViewPath = "view.firstRun";
    private const string MarkShownPath = "firstRun.markShown";
    private const string WelcomeNote = "Two questions so numbers and controls match your drone.";
    private const double BodyHeight = 520;
    private const double DialogWidth = 640;

    private readonly FrameworkElement _host;
    private bool _showing;

    public FirstRunDialog(FrameworkElement host)
    {
        _host = host;
        Live.Watch(host, ViewPath, Apply);
    }

    private void Apply(JsonObject view)
    {
        if (_showing || !MapUiJson.Flag(view["show"]) || _host.XamlRoot is not { } root) return;
        Present(root, view);
    }

    private async void Present(XamlRoot root, JsonObject view)
    {
        _showing = true;
        var dialog = FormLook.Dialog(root, "Welcome to Aircast QGC", Body(view), "Continue", "");
        dialog.Resources["ContentDialogMaxWidth"] = DialogWidth;
        await PromptQueue.Show(dialog, () => true);
        await FormWrites.Off(() => Bridge.Invoke(MarkShownPath));
        _showing = false;
    }

    private static UIElement Body(JsonObject view) => new ScrollViewer
    {
        MaxHeight = BodyHeight,
        VerticalScrollBarVisibility = ScrollBarVisibility.Auto,
        Content = FormLook.Stack(0,
            FormLook.Text(WelcomeNote, "FontSizeBody", "TextSecondaryBrush"),
            Units(MapUiJson.Text(view["unitsHeading"]), MapUiJson.Text(view["unitsDescription"])),
            new FormView(() => Bridge.Get(ViewPath), Preferences) { NotesFirst = true, Empty = () => new Border() }),
    };

    private static FormView Units(string heading, string description) =>
        new(() => Bridge.Get(SettingsCatalog.PagePath(SettingsCatalog.General)),
            page => SettingsCatalog.Sections(page)
                .Where(section => section.Group == SettingsCatalog.UnitsGroup)
                .Select(section => section with { Title = heading })
                .ToList())
        {
            Empty = () => new Border(),
            Hooks = new FormHooks
            {
                Replace = (form, section) => FormLook.Stack(4,
                    FormLook.Caption(description),
                    SettingsBodies.Units(form, section)),
            },
        };

    private static IReadOnlyList<FormSection> Preferences(JsonObject view) =>
        MapUiJson.Objects(view["vehiclePreferences"]).Select(FormControl.Parse).OfType<FormControl>().ToList() is { Count: > 0 } controls
            ? [new FormSection(MapUiJson.Text(view["vehicleHeading"]), "", MapUiJson.Text(view["vehicleDescription"]), "", [], [new FormBlock("", controls)])]
            : [];
}
