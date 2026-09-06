import Foundation

// QGC's QML settings are hand-authored pages, not one page per storage group:
// GeneralSettings.qml alone composes appSettings, unitsSettings and brandImageSettings
// under Appearance/Files/Units headings. Mirroring that page list keeps the operator's
// mental model; driving the sidebar straight off the C++ groups would expose the
// storage schema instead. Sections map whole groups, so nothing here invents a
// per-fact assignment that could drift from the metadata.
struct SettingsSection: Identifiable {
    let title: String
    let group: String
    var path: String { "settings.\(group)" }
    var id: String { path }
}

struct SettingsPage: Identifiable {
    let title: String
    let sections: [SettingsSection]
    // Some pages need more than a fact form: links are objects with their own
    // lifecycle, not settings.
    let showsLinks: Bool
    var id: String { title }

    init(title: String, sections: [SettingsSection], showsLinks: Bool = false) {
        self.title = title
        self.sections = sections
        self.showsLinks = showsLinks
    }

    static let all: [SettingsPage] = [
        SettingsPage(title: "General", sections: [
            .init(title: "Application", group: "appSettings"),
            .init(title: "Units", group: "unitsSettings"),
            .init(title: "Brand Image", group: "brandImageSettings"),
        ]),
        SettingsPage(title: "Fly View", sections: [
            .init(title: "Fly View", group: "flyViewSettings"),
            .init(title: "Battery Indicator", group: "batteryIndicatorSettings"),
            .init(title: "Gimbal Controller", group: "gimbalControllerSettings"),
        ]),
        SettingsPage(title: "Plan View", sections: [
            .init(title: "Plan View", group: "planViewSettings"),
        ]),
        SettingsPage(title: "Video", sections: [
            .init(title: "Video", group: "videoSettings"),
        ]),
        SettingsPage(title: "Maps", sections: [
            .init(title: "Maps", group: "mapsSettings"),
            .init(title: "Flight Map", group: "flightMapSettings"),
            .init(title: "Offline Maps", group: "offlineMapsSettings"),
        ]),
        SettingsPage(title: "Connections", sections: [
            .init(title: "Auto Connect", group: "autoConnectSettings"),
        ], showsLinks: true),
        SettingsPage(title: "MAVLink", sections: [
            .init(title: "MAVLink", group: "mavlinkSettings"),
            .init(title: "APM Stream Rates", group: "apmMavlinkStreamRateSettings"),
            .init(title: "Actions", group: "mavlinkActionsSettings"),
        ]),
        SettingsPage(title: "Flight Modes", sections: [
            .init(title: "Flight Modes", group: "flightModeSettings"),
        ]),
        SettingsPage(title: "ADSB Server", sections: [
            .init(title: "ADSB Server", group: "adsbVehicleManagerSettings"),
        ]),
        SettingsPage(title: "Packet Radio", sections: [
            .init(title: "Packet Radio", group: "packetRadioSettings"),
        ]),
        SettingsPage(title: "Remote ID", sections: [
            .init(title: "Remote ID", group: "remoteIDSettings"),
        ]),
        SettingsPage(title: "RTK GPS", sections: [
            .init(title: "RTK GPS", group: "rtkSettings"),
        ]),
        SettingsPage(title: "Firmware Upgrade", sections: [
            .init(title: "Firmware Upgrade", group: "firmwareUpgradeSettings"),
        ]),
        SettingsPage(title: "3D Viewer", sections: [
            .init(title: "3D Viewer", group: "viewer3DSettings"),
        ]),
    ]
}
