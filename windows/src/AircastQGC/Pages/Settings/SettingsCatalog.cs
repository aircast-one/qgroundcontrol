using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;
using AircastQGC.Forms;

namespace AircastQGC.Pages.Settings;

internal enum SettingsGroup { General, Safety, Control, Camera, Transmission, Device }

internal sealed record HelpLink(string Name, string Url, string Host);

internal sealed record PageLook(SettingsGroup Group, string Glyph);

internal sealed record SettingsEntry(
    string Title,
    bool ShowsLinks,
    bool ShowsAbout,
    bool ShowsVideoSources,
    bool ShowsPacketRadio,
    bool ShowsConsole,
    bool ShowsNtrip,
    bool ShowsPx4Logs,
    int SectionCount,
    IReadOnlyList<HelpLink> HelpLinks,
    string Keywords,
    SettingsGroup? Pilot = null)
{
    public string Shown => SettingsCatalog.PageTitle(Title);

    public PageLook Look => SettingsCatalog.Look(Title);

    public bool Matches(string needle) =>
        needle.Trim().ToLowerInvariant() is { Length: > 0 } wanted
        && (Title.ToLowerInvariant().Contains(wanted) || Shown.ToLowerInvariant().Contains(wanted) || Keywords.Contains(wanted));
}

internal static class SettingsCatalog
{
    public const string View = "view.settings";
    public const string General = "General";
    public const string Connections = "Connections";
    public const string FirmwareUpgrade = "Firmware Upgrade";
    public const string UnitsGroup = "unitsSettings";
    public const string FlyViewGroup = "flyViewSettings";
    public const string Viewer3DGroup = "viewer3DSettings";
    public const string MavlinkGroup = "mavlinkSettings";
    public const string MavlinkActionsGroup = "mavlinkActionsSettings";
    public const string RemoteIdGroup = "remoteIDSettings";
    public const string OfflineMapsGroup = "offlineMapsSettings";
    public const string SigningAfterBlock = "Ground Station";
    public const string GcsLocationBlock = "Ground Station Location";
    public const string MountpointBlock = "Mountpoint";
    public const string DevicePage = Device.DeviceLink.Page;

    private static readonly SettingsEntry FlasherEntry = new(Flasher.Page, false, false, false, false, false, false, false, 0, [], Flasher.Keywords);

    private static readonly SettingsEntry DeviceEntry = new(DevicePage, false, false, false, false, false, false, false, 0, [],
        "aircast device aircastd companion raspberry pi camera cameras wi-fi network tailscale recordings flight log telemetry engineering setup");

    public static readonly IReadOnlySet<string> GroupsWithAHeadEditor = new HashSet<string> { FlyViewGroup };

    public static readonly IReadOnlyList<(string Title, PageLook Look)> Looks =
    [
        ("General", new PageLook(SettingsGroup.General, "")),
        (Flasher.Page, new PageLook(SettingsGroup.General, "")),
        ("PX4 Log Transfer", new PageLook(SettingsGroup.General, "\uE896")),
        ("Firmware Upgrade", new PageLook(SettingsGroup.General, "")),
        ("App Logging", new PageLook(SettingsGroup.General, "")),
        ("Console", new PageLook(SettingsGroup.General, "")),
        ("About", new PageLook(SettingsGroup.General, "")),
        (PilotSettings.SafetyPage, new PageLook(SettingsGroup.Safety, "\uE80F")),
        ("ADSB Server", new PageLook(SettingsGroup.Safety, "")),
        ("Remote ID", new PageLook(SettingsGroup.Safety, "")),
        (PilotSettings.LimitsPage, new PageLook(SettingsGroup.Control, "\uEC4A")),
        ("Fly View", new PageLook(SettingsGroup.Control, "")),
        ("Flight Modes", new PageLook(SettingsGroup.Control, "")),
        ("Plan View", new PageLook(SettingsGroup.Control, "")),
        ("Maps", new PageLook(SettingsGroup.Control, "")),
        ("3D Viewer", new PageLook(SettingsGroup.Control, "")),
        ("Video", new PageLook(SettingsGroup.Camera, "")),
        ("Connections", new PageLook(SettingsGroup.Transmission, "")),
        ("Video sources", new PageLook(SettingsGroup.Transmission, "")),
        ("MAVLink", new PageLook(SettingsGroup.Transmission, "")),
        ("Packet Radio", new PageLook(SettingsGroup.Transmission, "")),
        ("RTK GPS", new PageLook(SettingsGroup.Transmission, "")),
        ("NTRIP / RTK", new PageLook(SettingsGroup.Transmission, "")),
        (DevicePage, new PageLook(SettingsGroup.Device, "")),
    ];

    private static readonly IReadOnlyDictionary<string, string> Titles = new Dictionary<string, string>
    {
        [Connections] = "Links",
        ["ADSB Server"] = "ADS-B server",
    };

    public static string PageTitle(string title) => Titles.TryGetValue(title, out var shown) ? shown : MapUiJson.SentenceCase(title);

    public static PageLook Look(string title) =>
        Looks.FirstOrDefault(l => l.Title == title).Look ?? new PageLook(SettingsGroup.General, "");

    public static string GroupTitle(SettingsGroup group) => group.ToString();

    public static string PagePath(string title) => $"{View}({title})";

    public static IReadOnlyList<SettingsEntry> Pages(JsonObject view) =>
        MapUiJson.Objects(view["pages"])
            .Select(Entry)
            .Where(page => page.Title.Length > 0)
            .Where(page => page.SectionCount > 0 || page.ShowsLinks || page.ShowsAbout || page.ShowsConsole || page.ShowsPx4Logs || page.ShowsVideoSources)
            .ToList() is { Count: > 0 } served
            ? Ordered(served.Concat(PilotSettings.Pages.Select(Pilot)).Append(FlasherEntry).Append(DeviceEntry))
            : [];

    private static SettingsEntry Pilot((string Title, SettingsGroup Group) page) =>
        new(page.Title, false, false, false, false, false, false, false, 0, [], PilotSettings.Keywords(page.Group), page.Group);

    private static IReadOnlyList<SettingsEntry> Ordered(IEnumerable<SettingsEntry> pages) =>
        pages
            .OrderBy(page => page.Look.Group)
            .ThenBy(page => Order(page.Title))
            .ToList();

    public static IEnumerable<IGrouping<SettingsGroup, SettingsEntry>> Grouped(IEnumerable<SettingsEntry> pages) =>
        pages.GroupBy(page => page.Look.Group);

    public static IReadOnlyList<FormSection> Sections(JsonObject page) =>
        FormParse.SettingsSections(page, group => group == MavlinkActionsGroup)
            .Select(section => GroupsWithAHeadEditor.Contains(section.Group) ? section with { Note = "" } : section)
            .ToList();

    public static IReadOnlyList<FormSection> Matches(string pageTitle, IReadOnlyList<FormSection> sections, string needle) =>
        needle.Trim().ToLowerInvariant() is not { Length: > 0 } wanted
            ? []
            : sections
                .Where(section => section.Group != UnitsGroup && section.Group != MavlinkActionsGroup)
                .Select(section => (Section: section, Hits: section.Controls.Where(c => Hit(c, wanted)).ToList()))
                .Where(hit => hit.Hits.Count > 0)
                .Select(hit => hit.Section with
                {
                    Title = hit.Section.Title.Length == 0 || hit.Section.Title.Equals(pageTitle, StringComparison.OrdinalIgnoreCase)
                        ? PageTitle(pageTitle)
                        : $"{PageTitle(pageTitle)} › {MapUiJson.SentenceCase(hit.Section.Title)}",
                    Note = "",
                    Blocks = [new FormBlock("", hit.Hits)],
                })
                .ToList();

    private static bool Hit(FormControl c, string wanted) =>
        c.Title.ToLowerInvariant().Contains(wanted) || c.Heading.ToLowerInvariant().Contains(wanted)
        || c.Name.ToLowerInvariant().Contains(wanted) || c.Keywords.Contains(wanted);

    private static int Order(string title) => Ranked(Looks.Select(l => l.Title).ToList().IndexOf(title));

    private static int Ranked(int at) => at < 0 ? int.MaxValue : at;

    private static SettingsEntry Entry(JsonObject page) => new(
        MapUiJson.Text(page["title"]),
        MapUiJson.Flag(page["showsLinks"]),
        MapUiJson.Flag(page["showsAbout"]),
        MapUiJson.Flag(page["showsVideoSources"]),
        MapUiJson.Flag(page["showsPacketRadio"]),
        MapUiJson.Flag(page["showsConsole"]),
        MapUiJson.Flag(page["showsNtrip"]),
        MapUiJson.Flag(page["showsPx4Logs"]),
        MapUiJson.Objects(page["sections"]).Count,
        MapUiJson.Objects(page["helpLinks"])
            .Select(link => new HelpLink(MapUiJson.Text(link["name"]), MapUiJson.Text(link["url"]), MapUiJson.Text(link["host"])))
            .ToList(),
        MapUiJson.Text(page["keywords"]));
}
