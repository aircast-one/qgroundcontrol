import Foundation

struct PageLink: Equatable {
    let label: String
    let href: String
}

struct GcsChoice: Equatable {
    let value: String
    let label: String
    let hides: [String]
}

struct ConnectAddress: Equatable {
    let label: String
    let url: String
}

struct ConnectTarget: Equatable {
    let proto: String
    let label: String
    let url: String
    let hint: String
    let tag: String
    let down: Bool
    let reach: String
    let qr: Bool
    let alternatives: [ConnectAddress]
}

struct ConnectCamera: Equatable {
    let name: String
    let status: DeviceBadge
    let notice: String
    let fix: PageLink?
    let targets: [ConnectTarget]
}

struct ConnectGroup: Equatable {
    let label: String
    let targets: [ConnectTarget]
}

struct ConnectAddresses: Equatable {
    let title: String
    let groups: [ConnectGroup]
}

struct ConnectTrouble: Equatable {
    let fallback: [ConnectTarget]
    let addresses: ConnectAddresses?
}

struct ConnectTelemetry: Equatable {
    let status: DeviceBadge
    let targets: [ConnectTarget]
}

struct ConnectView: Equatable {
    let choices: [GcsChoice]
    let scope: String
    let cameras: [ConnectCamera]
    let noCamera: PageLink?
    let telemetry: ConnectTelemetry?
    let trouble: ConnectTrouble?
}

private func parseLink(_ json: JSON?) -> PageLink? { json.map { PageLink(label: $0["label"].string, href: $0["href"].string) } }

private func parseTargets(_ json: JSON?, _ key: String) -> [ConnectTarget] {
    (json?[key].objects ?? []).map {
        ConnectTarget(
            proto: $0["proto"].string,
            label: $0["label"].string,
            url: $0["url"].string,
            hint: $0["hint"].string,
            tag: $0["tag"].string,
            down: $0["down"].bool,
            reach: $0["reach"].string,
            qr: $0["qr"].bool,
            alternatives: $0["alternatives"].objects.map { alt in ConnectAddress(label: alt["label"].string, url: alt["url"].string) }
        )
    }
}

private func parseTrouble(_ json: JSON?) -> ConnectTrouble? {
    guard let json else { return nil }
    return ConnectTrouble(
        fallback: parseTargets(json, "fallback"),
        addresses: json["addresses"].objectOrNil.map { addresses in
            ConnectAddresses(
                title: addresses["title"].string,
                groups: addresses["groups"].objects.map { group in ConnectGroup(label: group["label"].string, targets: parseTargets(group, "targets")) }
            )
        }
    )
}

func parseConnect(_ json: JSON) -> ConnectView? {
    guard json["choices"].arrayOrNil != nil else { return nil }
    return ConnectView(
        choices: json["choices"].objects.map { GcsChoice(value: $0["value"].string, label: $0["label"].string, hides: $0["hides"].strings) },
        scope: json["scope"].string,
        cameras: json["cameras"].objects.map { camera in
            ConnectCamera(
                name: camera["name"].string,
                status: parseBadge(camera["status"].objectOrNil),
                notice: camera["notice"].string,
                fix: parseLink(camera["fix"].objectOrNil),
                targets: parseTargets(camera, "targets")
            )
        },
        noCamera: parseLink(json["noCamera"].objectOrNil),
        telemetry: json["telemetry"].objectOrNil.map { telemetry in
            ConnectTelemetry(status: parseBadge(telemetry["status"].objectOrNil), targets: parseTargets(telemetry, "targets"))
        },
        trouble: parseTrouble(json["trouble"].objectOrNil)
    )
}

let GCS_KEY = "gcs"

func chosenGcs(_ choices: [GcsChoice], _ saved: String?) -> GcsChoice? { choices.first { $0.value == saved } ?? choices.first }

func shownTargets(_ targets: [ConnectTarget], _ choice: GcsChoice?) -> [ConnectTarget] {
    targets.filter { !(choice?.hides.contains($0.proto) ?? false) }
}

let REACH_RECHECK_MS = 30_000

func probeBlocked(_ url: String) -> Bool { url.isEmpty }

private let DEVICE_STORE = "device"

private var deviceStore: UserDefaults { UserDefaults(suiteName: DEVICE_STORE) ?? .standard }

func readStored(_ key: String) -> String? { deviceStore.string(forKey: key) }

func writeStored(_ key: String, _ value: String) { deviceStore.set(value, forKey: key) }
