import Foundation

struct DeviceStat: Equatable {
    let label: String
    let value: String
    let hint: String
    let level: Int
}

struct ServiceRow: Equatable {
    let label: String
    let state: String
    let level: Int
}

enum CloudStage {
    static let Linked = "linked"
    static let SigningIn = "signing-in"
    static let Connect = "connect"
}

struct CloudAccount: Equatable {
    let status: DeviceBadge
    let stage: String
    let deviceId: String
    let watchUrl: String
    let userCode: String
    let signInUrl: String
    let error: String
    let issuer: String
}

struct SoftwareNotice: Equatable {
    let level: Int
    let title: String
    let text: String
    let detailLabel: String
    let detail: String
}

struct SoftwareCandidate: Equatable {
    let title: String
    let notesUrl: String
    let install: String
    let confirm: String
    let success: String
}

struct SoftwareStatus: Equatable {
    let status: DeviceBadge
    let summary: String
    let notices: [SoftwareNotice]
    let candidate: SoftwareCandidate?
    let notes: [String]
    let checked: String
    let canCheck: Bool
}

struct HealthBadge: Equatable {
    let level: Int
    let text: String
    let href: String
}

struct DeviceSettingsView: Equatable {
    let health: HealthBadge
    let notice: DeviceBadge?
    let stats: [DeviceStat]
    let services: [ServiceRow]
    let cloud: CloudAccount
    let software: SoftwareStatus
    let hostname: String
}

private func parseCandidate(_ json: JSON?) -> SoftwareCandidate? {
    json.map {
        SoftwareCandidate(title: $0["title"].string, notesUrl: $0["notesUrl"].string, install: $0["install"].string, confirm: $0["confirm"].string, success: $0["success"].string)
    }
}

private func parseSoftware(_ json: JSON?) -> SoftwareStatus {
    SoftwareStatus(
        status: parseBadge(json?["status"].objectOrNil),
        summary: json?["summary"].string ?? "",
        notices: (json?["notices"].objects ?? []).map {
            SoftwareNotice(level: $0["level"].int(0), title: $0["title"].string, text: $0["text"].string, detailLabel: $0["detailLabel"].string, detail: $0["detail"].string)
        },
        candidate: parseCandidate(json?["candidate"].objectOrNil),
        notes: json?["notes"].strings ?? [],
        checked: json?["checked"].string ?? "",
        canCheck: json?["canCheck"].bool ?? false
    )
}

private func parseCloud(_ json: JSON?) -> CloudAccount {
    CloudAccount(
        status: parseBadge(json?["status"].objectOrNil),
        stage: (json?["stage"].string ?? "").ifEmpty(CloudStage.Connect),
        deviceId: json?["deviceId"].string ?? "",
        watchUrl: json?["watchUrl"].string ?? "",
        userCode: json?["userCode"].string ?? "",
        signInUrl: json?["signInUrl"].string ?? "",
        error: json?["error"].string ?? "",
        issuer: json?["issuer"].string ?? ""
    )
}

func parseDeviceSettings(_ json: JSON) -> DeviceSettingsView? {
    guard let health = json["health"].objectOrNil else { return nil }
    return DeviceSettingsView(
        health: HealthBadge(level: health["level"].int(0), text: health["text"].string, href: health["href"].string),
        notice: json["notice"].objectOrNil.map { parseBadge($0) },
        stats: json["stats"].objects.map { DeviceStat(label: $0["label"].string, value: $0["value"].string, hint: $0["hint"].string, level: $0["level"].int(0)) },
        services: json["services"].objects.map { ServiceRow(label: $0["label"].string, state: $0["state"].string, level: $0["level"].int(0)) },
        cloud: parseCloud(json["cloud"].objectOrNil),
        software: parseSoftware(json["software"].objectOrNil),
        hostname: json["hostname"].string
    )
}

let SERVICES = "Services"
