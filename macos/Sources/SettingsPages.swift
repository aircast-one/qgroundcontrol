import Foundation

struct SettingsSection: Identifiable {
    let title: String
    let group: String
    var path: String { "settings.\(group)" }
    var id: String { path }
}

struct SettingsPage: Identifiable {
    let title: String
    let sections: [SettingsSection]
    let showsLinks: Bool
    let showsAbout: Bool
    let showsVideoSources: Bool
    var id: String { title }

    init(title: String, sections: [SettingsSection], showsLinks: Bool = false,
         showsAbout: Bool = false, showsVideoSources: Bool = false) {
        self.title = title
        self.sections = sections
        self.showsLinks = showsLinks
        self.showsAbout = showsAbout
        self.showsVideoSources = showsVideoSources
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
        ], showsVideoSources: true),
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
        SettingsPage(title: "About", sections: [], showsAbout: true),
    ]
}
