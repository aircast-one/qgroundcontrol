import Foundation

struct DeviceAction: Equatable {
    let label: String
    let href: String
}

struct DeviceCheck: Equatable {
    let label: String
    let value: String
    let level: Int
    let href: String
    let action: String
}

struct DevicePage: Equatable {
    let title: String
    let href: String
    let icon: String
    let summary: String
    let needs: Bool
}

struct DeviceUplink: Equatable {
    let carrier: String
    let backup: String
    let rate: String
    let network: String
    let standby: String
    let trouble: String
    let data: String
    let charts: [DeviceChartView]
}

struct DeviceHealth: Equatable {
    let memory: String
    let warning: DeviceBadge?
    let charts: [DeviceChartView]
}

struct DeviceStatus: Equatable {
    let name: String
    let level: Int
    let headline: String
    let detail: String
    let action: DeviceAction?
    let setup: DeviceAction?
    let checks: [DeviceCheck]
    let pages: [DevicePage]
    let link: DeviceUplink?
    let health: DeviceHealth?
    let events: String
}

func parseDevicePage(_ json: JSON?) -> DevicePage {
    DevicePage(
        title: json?["title"].string ?? "",
        href: json?["href"].string ?? "",
        icon: json?["icon"].string ?? "",
        summary: json?["summary"].string ?? "",
        needs: json?["needs"].bool ?? false
    )
}

private func parseCharts(_ json: JSON) -> [DeviceChartView] { json["charts"].objects.compactMap { parseChart($0) } }

private func parseUplink(_ json: JSON?) -> DeviceUplink? {
    json.map {
        DeviceUplink(
            carrier: $0["carrier"].string,
            backup: $0["backup"].string,
            rate: $0["rate"].string,
            network: $0["network"].string,
            standby: $0["standby"].string,
            trouble: $0["trouble"].string,
            data: $0["data"].string,
            charts: parseCharts($0)
        )
    }
}

private func parseHealth(_ json: JSON?) -> DeviceHealth? {
    json.map { DeviceHealth(memory: $0["memory"].string, warning: $0["warning"].objectOrNil.map { parseBadge($0) }, charts: parseCharts($0)) }
}

private func deviceAction(_ json: JSON?) -> DeviceAction? {
    guard let json, !json["href"].string.isEmpty else { return nil }
    return DeviceAction(label: json["label"].string, href: json["href"].string)
}

func parseDeviceStatus(_ json: JSON) -> DeviceStatus? {
    guard !json["headline"].string.isEmpty else { return nil }
    return DeviceStatus(
        name: json["name"].string,
        level: json["level"].int(0),
        headline: json["headline"].string,
        detail: json["detail"].string,
        action: deviceAction(json["action"].objectOrNil),
        setup: deviceAction(json["setup"].objectOrNil),
        checks: json["checks"].objects.map {
            DeviceCheck(label: $0["label"].string, value: $0["value"].string, level: $0["level"].int(0), href: $0["href"].string, action: $0["action"].string)
        },
        pages: json["pages"].objects.map { parseDevicePage($0) },
        link: parseUplink(json["link"].objectOrNil),
        health: parseHealth(json["health"].objectOrNil),
        events: json["events"].string
    )
}

private let PAGE_ICONS: [String: Icon] = [
    "qr-code": .link,
    "video": .videocam,
    "sliders-horizontal": .tune,
    "crosshair": .centerFocus,
    "network": .lan,
    "activity": .analytics,
    "gauge": .stopCircle,
    "clock": .history,
    "settings": .developerBoard,
    "server": .terminal,
    "scan-search": .build,
]

func devicePageIcon(_ icon: String) -> Icon { PAGE_ICONS[icon] ?? .settings }

func deviceNeedsAttention(_ status: DeviceStatus?) -> Bool { status.map { $0.level != DeviceLevel.Ready } ?? false }
