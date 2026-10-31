using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text.Json.Nodes;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Pages.Settings;

internal static class SettingsBodies
{
    public const double DetailMaxWidth = 880;

    private const string UnitSystemPath = "settings.unitsSettings.unitSystem";
    private const string SetUnitSystem = "settings.unitsSettings.setUnitSystem";
    private const string OsmFileSetting = "settings.viewer3DSettings.osmFilePath";
    private const string OsmEnabledSetting = "settings.viewer3DSettings.enabled";
    private const string AdvancedUiView = "view.advancedUi";
    private const string SetAdvancedUi = "advancedUi.set";
    private const string VideoSourcesHeading = "Video sources";

    private static readonly string[] UnitSystems = ["Metric", "Imperial", "Custom"];
    private static readonly (string Title, string Page, string Keywords)[] HeadEditors =
    [
        ("On-screen RC controls", "Fly View", "on-screen onscreen rc controls remote channel override slider button switch momentary"),
    ];
    private static readonly string[] PresetUnits = ["horizontalDistanceUnits", "verticalDistanceUnits", "areaUnits", "speedUnits", "temperatureUnits"];

    public static UIElement Body(SettingsEntry page, SettingsNav nav) =>
        page.Title == SettingsCatalog.DevicePage ? Device.DeviceScreens.Home(nav)
        : page.Title == Flasher.Page ? new FlasherPage()
        : page.Pilot is { } group ? Scrolled(PilotSettings.Tab(group, nav.Open))
        : page.ShowsLinks ? new LinksPanel(nav, Form(page, nav))
        : page.ShowsAbout ? Scrolled(About(page.HelpLinks))
        : page.ShowsConsole ? new AppLogPanel()
        : page.ShowsPx4Logs ? new Border { Padding = new Thickness(16, 0, 16, 0), Child = new Px4LogTransfer() }
        : page.ShowsVideoSources && page.SectionCount > 0 ? Scrolled(FormLook.Stack(0, FormLook.Heading(VideoSourcesHeading), new CamerasPanel(), Form(page, nav)))
        : page.ShowsVideoSources ? Scrolled(new CamerasPanel())
        : page.Title == SettingsCatalog.FirmwareUpgrade ? new Analyze.FirmwareTool()
        : Scrolled(Form(page, nav));

    public static FormView Form(SettingsEntry page, SettingsNav nav) => new(() => Bridge.Get(SettingsCatalog.PagePath(page.Title)), SettingsCatalog.Sections)
    {
        PageTitle = page.Title,
        Hooks = Hooks(page, nav),
    };

    public static ScrollViewer Scrolled(UIElement content) => new()
    {
        VerticalScrollBarVisibility = ScrollBarVisibility.Auto,
        Content = new Grid
        {
            Padding = new Thickness(24, 0, 24, 24),
            ColumnDefinitions = { new ColumnDefinition { MaxWidth = DetailMaxWidth } },
            Children = { content },
        },
    };

    public static UIElement Search(string query, IReadOnlyList<SettingsEntry> pages, Action<string> open)
    {
        var pageHits = pages.Where(p => p.Matches(query)).ToList();
        var editorHits = EditorHits(query, pages);
        var pilotHits = PilotSettings.SearchHits(query);
        var searched = pages.Where(p => p.SectionCount > 0).Select(p => p.Title).ToList();
        var form = new FormView(
            () => new JsonObject
            {
                ["pages"] = new JsonArray(searched
                    .Select(title => (JsonNode)new JsonObject { ["title"] = title, ["page"] = Bridge.Get(SettingsCatalog.PagePath(title)) })
                    .ToArray()),
            },
            raw => MapUiJson.Objects(raw["pages"])
                .SelectMany(hit => SettingsCatalog.Matches(MapUiJson.Text(hit["title"]),
                    SettingsCatalog.Sections(hit["page"] as JsonObject ?? new JsonObject()), query))
                .ToList())
        {
            LoadingText = "Searching settings.",
            Empty = () => pageHits.Count > 0 || pilotHits.Count > 0 || editorHits.Count > 0 ? new Border() : FormLook.Footnote($"No settings match “{query.Trim()}”."),
        };
        return Scrolled(FormLook.Stack(0,
            pageHits.Count > 0 ? FormLook.Heading("Pages") : null,
            pageHits.Count > 0 ? FormLook.Card(pageHits.Select(p => PageRow(p, open))) : null,
            pilotHits.Count > 0 ? new PilotSettingsPanel(pilotHits, open) : null,
            editorHits.Count > 0 ? FormLook.Heading("Editors") : null,
            editorHits.Count > 0 ? FormLook.Card(editorHits.Select(hit => EditorRow(hit.Title, hit.Page, open))) : null,
            form));
    }

    private static IReadOnlyList<(string Title, SettingsEntry Page)> EditorHits(string query, IReadOnlyList<SettingsEntry> pages) =>
        query.Trim().ToLowerInvariant() is not { Length: > 0 } wanted
            ? []
            : HeadEditors
                .Where(editor => editor.Title.ToLowerInvariant().Contains(wanted) || editor.Keywords.Contains(wanted))
                .SelectMany(editor => pages.Where(page => page.Title == editor.Page).Select(page => (editor.Title, page)))
                .ToList();

    private static UIElement EditorRow(string title, SettingsEntry page, Action<string> open)
    {
        var row = new Button
        {
            HorizontalAlignment = HorizontalAlignment.Stretch,
            HorizontalContentAlignment = HorizontalAlignment.Stretch,
            Padding = new Thickness(0),
            Content = FormLook.Row(FormLook.Line(12, FormLook.Glyph(page.Look.Glyph, 16), FormLook.Stack(0, FormLook.Text(title), FormLook.Caption(page.Shown))), FormLook.Glyph("\uE76C", 12)),
        };
        FormLook.Paint(row, ButtonLook.Subtle);
        AutomationProperties.SetName(row, title);
        row.Click += (_, _) => open(page.Title);
        return row;
    }

    private static UIElement PageRow(SettingsEntry page, Action<string> open)
    {
        var row = new Button
        {
            HorizontalAlignment = HorizontalAlignment.Stretch,
            HorizontalContentAlignment = HorizontalAlignment.Stretch,
            Padding = new Thickness(0),
            Content = FormLook.Row(FormLook.Line(12, FormLook.Glyph(page.Look.Glyph, 16), FormLook.Text(page.Shown)), FormLook.Glyph("", 12)),
        };
        FormLook.Paint(row, ButtonLook.Subtle);
        AutomationProperties.SetName(row, page.Shown);
        row.Click += (_, _) => open(page.Title);
        return row;
    }

    private static UIElement FlasherRow(SettingsNav nav)
    {
        var row = FormLook.Stretched(
            FormLook.Row(FormLook.Line(12, FormLook.Glyph(SettingsCatalog.Look(Flasher.Page).Glyph, 16),
                FormLook.Stack(2, FormLook.Text(Flasher.Page), FormLook.Caption(Flasher.Glance))), FormLook.Glyph("", 12)),
            Flasher.Page, () => nav.Open(Flasher.Page));
        ToolTipService.SetToolTip(row, Flasher.Glance);
        return FormLook.Margined(FormLook.Card(row), new Thickness(0, 0, 0, 8));
    }

    private static FormHooks Hooks(SettingsEntry page, SettingsNav nav) => new()
    {
        Top = form => page.ShowsNtrip ? new NtripStatusPanel(form.Reload)
            : page.ShowsPacketRadio ? new PacketRadioPanel(form.Reload)
            : page.Title == SettingsCatalog.General ? FlasherRow(nav)
            : null,
        Replace = (form, section) => section.Group switch
        {
            SettingsCatalog.UnitsGroup => Units(form, section),
            SettingsCatalog.MavlinkActionsGroup => new MavlinkActionsPanel(form.Reload),
            _ => null,
        },
        AfterBlock = (form, section, block) =>
            page.ShowsNtrip && block.Title == SettingsCatalog.MountpointBlock ? new MountpointBrowser(form.Reload)
            : section.Group == SettingsCatalog.RemoteIdGroup && block.Title == SettingsCatalog.GcsLocationBlock ? new GcsPositionPanel()
            : section.Group == SettingsCatalog.MavlinkGroup && block.Title == SettingsCatalog.SigningAfterBlock ? new SigningKeysPanel()
            : null,
        AfterSection = (form, section) => section.Group switch
        {
            SettingsCatalog.FlyViewGroup => new RcControlsEditor(),
            SettingsCatalog.Viewer3DGroup => OsmPicker(form),
            SettingsCatalog.OfflineMapsGroup => new OfflineMapsPanel(nav),
            SettingsCatalog.MavlinkGroup => new LinkStatusPanel(),
            _ => null,
        },
        Bottom = _ => page.Title == SettingsCatalog.General ? FormLook.Stack(0, new AppUpdatePanel(), new ResetAllSettingsRow()) : null,
    };

    internal static UIElement? Units(FormView form, FormSection section)
    {
        if (MapUiJson.Number(Bridge.Get(UnitSystemPath)["value"]) is not { } read) return null;
        var system = Math.Clamp((int)read, 0, UnitSystems.Length - 1);
        var custom = system == UnitSystems.Length - 1;
        var choice = new RadioButtons { ItemsSource = UnitSystems, SelectedIndex = system, MaxColumns = UnitSystems.Length };
        AutomationProperties.SetName(choice, "Measurement system");
        choice.SelectionChanged += async (_, _) =>
        {
            var picked = choice.SelectedIndex;
            if (picked < 0 || picked == system) return;
            await FormWrites.Off(() => FormWrites.Ok(SetUnitSystem, picked));
            form.Reload();
        };
        var rows = section.Controls
            .Where(c => c.Name != "customUnits" && c.Name != "weightUnits")
            .Where(c => custom || !PresetUnits.Contains(c.Name))
            .ToList();
        return FormLook.Stack(4,
            FormLook.Card(FormLook.Row(FormLook.Text("Measurement system"), choice)),
            FormLook.Footnote(custom
                ? "Each measurement is set on its own below."
                : $"Distance, area, speed and temperature all follow {UnitSystems[system]}. Choose Custom to set them one at a time."),
            rows.Count > 0 ? form.BlockCard(new FormBlock("", rows)) : null);
    }

    private static UIElement OsmPicker(FormView form)
    {
        var refusal = FormLook.Caption("", "CriticalBrush");
        var enabled = form.Sections.SelectMany(s => s.Controls).FirstOrDefault(c => c.Path == OsmEnabledSetting)?.On ?? false;
        var button = FormLook.Button("Choose OpenStreetMap file", async () =>
        {
            var path = await FilePicks.Open(form.XamlRoot, [".osm", ".xml"]);
            if (path is null) return;
            var refused = await FormWrites.Off(() => FormWrites.Set(OsmFileSetting, path));
            FormLook.Say(refusal, refused);
            if (refused is null) form.Reload();
        });
        button.IsEnabled = enabled;
        return FormLook.Margined(FormLook.Stack(4, button, refusal), new Thickness(0, 12, 0, 0));
    }

    private static UIElement About(IReadOnlyList<HelpLink> links)
    {
        var shown = $"Aircast {AppUpdates.Version} {RuntimeInformation.ProcessArchitecture.ToString().ToUpperInvariant()}";
        var versionRow = new Button
        {
            HorizontalAlignment = HorizontalAlignment.Stretch,
            HorizontalContentAlignment = HorizontalAlignment.Stretch,
            Padding = new Thickness(0),
            Content = FormLook.Row(FormLook.Text("Aircast version"), FormLook.Text(shown, "FontSizeBody", "TextSecondaryBrush")),
        };
        FormLook.Paint(versionRow, ButtonLook.Subtle);
        AutomationProperties.SetName(versionRow, $"Aircast version {shown}");
        versionRow.Click += (_, _) => FormWrites.Off(() =>
            FormWrites.Ok(SetAdvancedUi, !MapUiJson.Flag(Bridge.Get(AdvancedUiView)["shown"], true)));
        var refusal = FormLook.Caption("", "CriticalBrush");
        return FormLook.Stack(0,
            FormLook.Heading("About"),
            FormLook.Card(versionRow),
            FormLook.Heading("Support"),
            FormLook.Card(links.Select(link => (UIElement)FormLook.Row(
                FormLook.Stack(2, FormLook.Text(link.Name), FormLook.Caption(link.Host, "AccentTextBrush")),
                OpenButton(link, refusal)))),
            FormLook.Margined(refusal, new Thickness(4, 8, 4, 0)));
    }

    private static Button OpenButton(HelpLink link, TextBlock refusal)
    {
        var button = FormLook.Button("Open", async () => FormLook.Say(refusal, await Launch(link)));
        AutomationProperties.SetName(button, $"Open {link.Name}");
        return button;
    }

    private static async System.Threading.Tasks.Task<string?> Launch(HelpLink link)
    {
        if (!Uri.TryCreate(link.Url, UriKind.Absolute, out var uri)) return $"{link.Name} has no address to open.";
        try { return await Windows.System.Launcher.LaunchUriAsync(uri) ? null : $"Windows could not open {link.Host}."; }
        catch (Exception error) { return $"Windows could not open {link.Host}: {error.Message}"; }
    }
}

internal sealed class ResetAllSettingsRow : UserControl
{
    private const string ClearNextBoot = "settings.appSettings.clearSettingsNextBoot";

    public ResetAllSettingsRow()
    {
        Margin = new Thickness(0, 24, 0, 0);
        Loaded += (_, _) => Render(FormJson.Truthy(Bridge.Get(ClearNextBoot)["value"]));
    }

    private void Render(bool pending) => Content = FormLook.Card(FormLook.Row(
        FormLook.Stack(2,
            FormLook.Text("Reset all settings"),
            FormLook.Caption(pending ? "All settings will be cleared on next start" : "Clears every application setting the next time Aircast starts.")),
        FormLook.Button(pending ? "Cancel reset" : "Reset all settings…", () => Toggle(pending))));

    private async void Toggle(bool pending)
    {
        if (!pending && !await FormLook.Confirm(XamlRoot, "Reset all settings",
                "All settings will be cleared the next time Aircast starts. This cannot be undone.", "OK")) return;
        await FormWrites.Off(() => FormWrites.Set(ClearNextBoot, !pending));
        Render(FormJson.Truthy(await FormWrites.Off(() => Bridge.Get(ClearNextBoot)["value"])));
    }
}
