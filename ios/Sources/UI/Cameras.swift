import Foundation

let CAMERAS_VIEW = "view.cameras"
let VIDEO_SOURCES_PAGE = "Video sources"
private let CAMERA_GROUP_DEVICE = "This device"
private let FROM_THE_DRONE = "From the drone"

enum CameraStatus: CaseIterable, Equatable {
    case Live, Connecting, NoSignal, Idle

    var token: String {
        switch self {
        case .Live: "live"
        case .Connecting: "connecting"
        case .NoSignal: "noSignal"
        case .Idle: "idle"
        }
    }

    var label: String {
        switch self {
        case .Live: "Live"
        case .Connecting: "Connecting"
        case .NoSignal: "No signal"
        case .Idle: "Not playing"
        }
    }
}

struct CameraKind: Equatable, Hashable {
    let raw: String
    let label: String
    let group: String
    let needsUrl: Bool
    let hint: String
}

struct CameraEntry: Equatable, Identifiable {
    let slot: Int
    let stored: Int?
    let title: String
    let short: String
    let name: String
    let source: String
    let url: String
    let problem: String?
    let fromDrone: Bool
    let active: Bool
    let status: CameraStatus

    var id: Int { slot }
}

struct CameraPip: Equatable {
    let enabled: Bool
    let slot: Int?
}

struct CamerasReading: Equatable {
    let readable: Bool
    let reason: String
    let pip: CameraPip
    let cameras: [CameraEntry]
    let kinds: [CameraKind]

    var stored: [CameraEntry] { cameras.filter { $0.stored != nil } }
}

struct CameraGuess: Equatable {
    let kind: String?
    let choices: [String]
    let ambiguous: Bool
    let problem: String?
}

private func objects(_ json: JSON, _ key: String) -> [JSON] {
    json[key].array.filter { $0.object != nil }
}

private func textOrNull(_ json: JSON, _ key: String) -> String? {
    json.has(key) ? json[key].stringOrNil.flatMap { $0.isBlank ? nil : $0 } : nil
}

private func intOrNull(_ json: JSON, _ key: String) -> Int? {
    json.has(key) ? json[key].int(0) : nil
}

func camerasReading(_ view: JSON?) -> CamerasReading? {
    guard let view, view["class"].string == "Cameras" else { return nil }
    let pip = view["pip"].object != nil ? view["pip"] : nil
    return CamerasReading(
        readable: view["readable"].bool(true),
        reason: view["reason"].string,
        pip: CameraPip(enabled: pip?["enabled"].bool == true, slot: pip.flatMap { intOrNull($0, "slot") }),
        cameras: objects(view, "cameras").map { camera in
            CameraEntry(
                slot: camera["slot"].int(0),
                stored: intOrNull(camera, "stored"),
                title: camera["title"].string,
                short: camera["short"].string,
                name: camera["name"].string,
                source: camera["source"].string,
                url: camera["url"].string,
                problem: textOrNull(camera, "problem"),
                fromDrone: camera["fromDrone"].bool,
                active: camera["active"].bool,
                status: CameraStatus.allCases.first { $0.token == camera["status"].string } ?? .Idle
            )
        },
        kinds: objects(view, "kinds").map { kind in
            CameraKind(
                raw: kind["raw"].string,
                label: kind["label"].string,
                group: kind["group"].string,
                needsUrl: kind["needsUrl"].bool,
                hint: kind["hint"].string
            )
        }
    )
}

func cameraGuess(_ reply: JSON?) -> CameraGuess? {
    guard let reply, reply["ok"].bool else { return nil }
    return CameraGuess(
        kind: textOrNull(reply, "kind"),
        choices: reply["choices"].array.map(\.string),
        ambiguous: reply["ambiguous"].bool,
        problem: textOrNull(reply, "problem")
    )
}

func keptGuess(_ guess: CameraGuess?, _ source: String, _ address: String, _ keptAddress: String?) -> CameraGuess? {
    guard let guess, address.trimmed == keptAddress, !guess.choices.contains(source) else { return guess }
    return CameraGuess(kind: source, choices: [source], ambiguous: false, problem: nil)
}

func chosenKind(_ guess: CameraGuess?, _ picked: String?, _ fallback: String) -> String {
    picked.flatMap { pick in guess.map { $0.choices.contains(pick) } == true ? pick : nil } ?? guess?.kind ?? fallback
}

func guessText(_ guess: CameraGuess?) -> String {
    guard let guess else { return "" }
    if let problem = guess.problem { return problem }
    if guess.ambiguous { return "Which kind of stream is it?" }
    return guess.kind.map { "\(kindLabel($0)) stream" } ?? ""
}

func cameraDetail(_ camera: CameraEntry) -> String {
    camera.fromDrone ? FROM_THE_DRONE : !camera.url.isBlank ? camera.url : kindLabel(camera.source)
}

func otherSources(_ reading: CamerasReading?) -> [CameraKind] {
    let open = (reading?.kinds ?? []).filter { kind in
        !kind.needsUrl && !(reading?.stored ?? []).contains { $0.source == kind.raw }
    }
    return open.filter { $0.group == CAMERA_GROUP_DEVICE } + open.filter { $0.group != CAMERA_GROUP_DEVICE }
}

func deviceCameraKind(_ kind: CameraKind) -> Bool { kind.group == CAMERA_GROUP_DEVICE }

func otherSourceLabel(_ kind: CameraKind) -> String {
    kind.group == CAMERA_GROUP_DEVICE ? "This phone's \(kindLabel(kind.label).lowercased())" : kindLabel(kind.label)
}

func kindLabel(_ label: String) -> String {
    sentenceCase(label.removingSuffix(" Video Stream").ifBlank(label))
}

func camerasGlance(_ reading: CamerasReading?) -> String {
    let cameras = reading?.cameras ?? []
    guard let active = cameras.first(where: \.active) ?? cameras.first else { return "" }
    return cameras.count == 1 ? active.title : "\(active.title) · \(cameras.count) cameras"
}
