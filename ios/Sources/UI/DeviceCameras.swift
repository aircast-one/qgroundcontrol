import Foundation

struct CameraCopy: Equatable {
    let label: String
    let text: String
    let qr: Bool
}

struct CameraFact: Equatable {
    let label: String
    let value: String
}

struct CameraFix: Equatable {
    let label: String
    let scan: Bool
}

struct CameraLinkChip: Equatable {
    let key: String
    let label: String
    let level: Int
    let href: String
}

struct SetupPart: Equatable {
    let title: String
    let summary: String
    let badge: DeviceBadge?
    let open: Bool
}

struct NoticeLink: Equatable {
    let label: String
    let href: String
}

struct LinkedNotice: Equatable {
    let level: Int
    let text: String
    let link: NoticeLink?
}

struct ServerDest: Equatable {
    let kind: String
    let part: SetupPart
    let on: Bool
    let url: String
    let key: String
    let secret: String
    let app: String
    let track: String
    let watch: String
}

struct CameraShareLink: Equatable {
    let id: String
    let label: String
    let url: String
    let expires: String
    let dead: String
}

struct CameraWatch: Equatable {
    let part: SetupPart
    let cloud: Bool
    let canConnect: Bool
    let signIn: Bool
    let cloudLinks: [DeviceEndpoint]
    let cloudCopy: String
    let links: [DeviceEndpoint]
}

struct CameraRadio: Equatable {
    let part: SetupPart
    let notice: LinkedNotice
    let form: Bool
    let address: String
    let stop: Bool
}

struct CameraServers: Equatable {
    let part: SetupPart
    let dests: [ServerDest]
}

struct CameraShares: Equatable {
    let part: SetupPart
    let links: [CameraShareLink]
    let hours: [DeviceOption]
}

struct CameraDetect: Equatable {
    let part: SetupPart
    let on: Bool
    let model: String
    let notice: LinkedNotice?
}

struct CameraRecord: Equatable {
    let part: SetupPart
    let on: Bool
    let full: Bool
    let usage: String
}

struct CameraGimbal: Equatable {
    let part: SetupPart
    let owner: Bool
}

struct CameraSetup: Equatable {
    let watch: CameraWatch
    let radio: CameraRadio
    let servers: CameraServers
    let shares: CameraShares
    let detect: CameraDetect
    let record: CameraRecord
    let gimbal: CameraGimbal?
}

struct CameraView: Equatable {
    let name: String
    let status: DeviceBadge
    var live: Bool
    let shared: Bool
    let preview: String
    let placeholder: String
    let watch: String
    let copies: [CameraCopy]
    let facts: [CameraFact]
    let bitrate: DeviceChartView?
    let diagnosis: DeviceBadge?
    let fix: CameraFix?
    let links: [DeviceEndpoint]
    let chips: [CameraLinkChip]
    let recording: Bool
    let wizard: CameraWizardView
    let setup: CameraSetup
    let topic: DevicePage
}

struct CamerasView: Equatable {
    let cameras: [CameraView]
    let add: CameraWizardView
    let cloudLink: String
    let video: DevicePage
}

func parseSetupPart(_ json: JSON?) -> SetupPart {
    SetupPart(
        title: json?["title"].string ?? "",
        summary: json?["summary"].string ?? "",
        badge: optDeviceBadge(json, "badge"),
        open: json?["open"].bool ?? false
    )
}

func parseLinkedNotice(_ json: JSON?) -> LinkedNotice? {
    guard let json else { return nil }
    let badge = parseBadge(json)
    let link = json["link"].objectOrNil.map { NoticeLink(label: $0["label"].string, href: $0["href"].string) }
    return LinkedNotice(level: badge.level, text: badge.text, link: link)
}

private let EMPTY_NOTICE = LinkedNotice(level: DeviceLevel.Neutral, text: "", link: nil)

private func partOf(_ json: JSON?) -> SetupPart { parseSetupPart(json?["part"].objectOrNil) }

private func parseCameraSetup(_ json: JSON?) -> CameraSetup {
    let watch = json?["watch"].objectOrNil
    let radio = json?["radio"].objectOrNil
    let servers = json?["servers"].objectOrNil
    let shares = json?["shares"].objectOrNil
    let detect = json?["detect"].objectOrNil
    let record = json?["record"].objectOrNil
    return CameraSetup(
        watch: CameraWatch(
            part: partOf(watch),
            cloud: watch?["cloud"].bool ?? false,
            canConnect: watch?["canConnect"].bool ?? false,
            signIn: watch?["signIn"].bool ?? false,
            cloudLinks: parseEndpointList(watch, "cloudLinks"),
            cloudCopy: watch?["cloudCopy"].string ?? "",
            links: parseEndpointList(watch, "links")
        ),
        radio: CameraRadio(
            part: partOf(radio),
            notice: parseLinkedNotice(radio?["notice"].objectOrNil) ?? EMPTY_NOTICE,
            form: radio?["form"].bool ?? false,
            address: radio?["address"].string ?? "",
            stop: radio?["stop"].bool ?? false
        ),
        servers: CameraServers(
            part: partOf(servers),
            dests: (servers?["dests"].objects ?? []).map {
                ServerDest(
                    kind: $0["kind"].string,
                    part: partOf($0),
                    on: $0["on"].bool,
                    url: $0["url"].string,
                    key: $0["key"].string,
                    secret: $0["secret"].string,
                    app: $0["app"].string,
                    track: $0["track"].string,
                    watch: $0["watch"].string
                )
            }
        ),
        shares: CameraShares(
            part: partOf(shares),
            links: (shares?["links"].objects ?? []).map {
                CameraShareLink(id: $0["id"].string, label: $0["label"].string, url: $0["url"].string, expires: $0["expires"].string, dead: $0["dead"].string)
            },
            hours: parseOptions(shares, "hours")
        ),
        detect: CameraDetect(
            part: partOf(detect),
            on: detect?["on"].bool ?? false,
            model: detect?["model"].string ?? "",
            notice: parseLinkedNotice(detect?["notice"].objectOrNil)
        ),
        record: CameraRecord(part: partOf(record), on: record?["on"].bool ?? false, full: record?["full"].bool ?? false, usage: record?["usage"].string ?? ""),
        gimbal: json?["gimbal"].objectOrNil.map { CameraGimbal(part: partOf($0), owner: $0["owner"].bool) }
    )
}

private func parseCamera(_ json: JSON) -> CameraView? {
    guard let wizard = parseCameraWizard(json["wizard"].objectOrNil) else { return nil }
    return CameraView(
        name: json["name"].string,
        status: parseBadge(json["status"].objectOrNil),
        live: json["live"].bool,
        shared: json["shared"].bool,
        preview: json["preview"].string,
        placeholder: json["placeholder"].string,
        watch: json["watch"].string,
        copies: json["copies"].objects.map { CameraCopy(label: $0["label"].string, text: $0["text"].string, qr: $0["qr"].bool) },
        facts: json["facts"].objects.map { CameraFact(label: $0["label"].string, value: $0["value"].string) },
        bitrate: parseChart(json["bitrate"].objectOrNil),
        diagnosis: optDeviceBadge(json, "diagnosis"),
        fix: json["fix"].objectOrNil.map { CameraFix(label: $0["label"].string, scan: $0["scan"].bool) },
        links: parseEndpointList(json, "links"),
        chips: json["chips"].objects.map { CameraLinkChip(key: $0["key"].string, label: $0["label"].string, level: $0["level"].int(0), href: $0["href"].string) },
        recording: json["recording"].bool,
        wizard: wizard,
        setup: parseCameraSetup(json["setup"].objectOrNil),
        topic: parseDevicePage(json["topic"].objectOrNil)
    )
}

func parseCameras(_ json: JSON) -> CamerasView? {
    guard let add = parseCameraWizard(json["add"].objectOrNil) else { return nil }
    return CamerasView(
        cameras: json["cameras"].objects.compactMap { parseCamera($0) },
        add: add,
        cloudLink: json["cloudLink"].string,
        video: parseDevicePage(json["video"].objectOrNil)
    )
}

enum CamerasShown: Equatable {
    case Add
    case Wall
    case Camera(CameraView)
}

func camerasShown(_ cameras: [CameraView], _ picked: String, _ adding: Bool) -> CamerasShown {
    if adding || cameras.isEmpty { return .Add }
    if let named = cameras.first(where: { $0.name == picked }) { return .Camera(named) }
    if cameras.count == 1, let only = cameras.first { return .Camera(only) }
    return .Wall
}

func cameBack(_ before: [String: Bool], _ cameras: [CameraView]) -> [String] {
    cameras.filter { $0.live && before[$0.name] == false }.map(\.name)
}

func liveness(_ cameras: [CameraView]) -> [String: Bool] {
    Dictionary(cameras.map { ($0.name, $0.live) }, uniquingKeysWith: { _, last in last })
}

func cloudflareTarget(_ app: String, _ track: String) -> String {
    let app = app.trimmingCharacters(in: .whitespacesAndNewlines)
    let track = track.trimmingCharacters(in: .whitespacesAndNewlines)
    return !app.isEmpty && !track.isEmpty ? "cloudflare://\(app)/\(track)" : ""
}

enum CameraSetupSection {
    static let Watch = "cam-watch"
    static let Radio = "cam-radio"
    static let Servers = "cam-servers"
    static let Shares = "cam-share"
    static let Detect = "cam-detect"
    static let Record = "cam-record"
    static let Gimbal = "cam-gimbal"
}

func openedSection(_ href: String) -> String {
    guard let hash = href.firstIndex(of: "#") else { return "" }
    return String(href[href.index(after: hash)...])
}

func cameraSectionHref(_ camera: String, _ section: String) -> String { "/settings/camera?camera=\(uriComponent(camera))#\(section)" }
