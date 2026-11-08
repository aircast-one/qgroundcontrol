import Foundation

struct BackendChoice: Equatable {
    let status: DeviceBadge
    let text: String
    let switchTo: String
    let label: String
    let confirm: String
    let success: String
}

struct SignInMode: Equatable {
    let value: String
    let title: String
    let desc: String
    let fields: [String]
    let clears: [String]
    let submit: String
    let unsaved: String
    let confirm: String
}

struct SignInField: Equatable {
    var name: String
    var label: String
    var value: String
    var placeholder: String
    var secret: Bool
}

struct PublishSignIn: Equatable {
    var status: DeviceBadge
    var awaitingCloud: Bool
    var mode: String
    var modes: [SignInMode]
    var fields: [SignInField]
    var environment: String
    var verdict: DeviceBadge
}

struct DiagnosticsOffer: Equatable {
    let title: String
    let text: String
    let endpoint: String
    let accept: String
    let dismiss: String
}

func parseOffer(_ json: JSON?) -> DiagnosticsOffer? {
    guard let json, json.objectOrNil != nil else { return nil }
    return DiagnosticsOffer(title: json["title"].string ?? "", text: json["text"].string ?? "", endpoint: json["endpoint"].string ?? "", accept: json["accept"].string ?? "", dismiss: json["dismiss"].string ?? "")
}

struct DiagnosticsSettings: Equatable {
    let status: DeviceBadge
    let hint: String
    let endpoint: String
    let token: String
    let tokenLabel: String
    let streaming: Bool
    let canStream: Bool
    let state: String
    let confirm: String
    let offer: DiagnosticsOffer?
}

struct EngineeringView: Equatable {
    let backend: BackendChoice
    let publish: PublishSignIn
    let diagnostics: DiagnosticsSettings
}

private func parsePublish(_ json: JSON?) -> PublishSignIn {
    PublishSignIn(
        status: parseBadge(json?["status"].objectOrNil),
        awaitingCloud: json?["awaitingCloud"].bool ?? false,
        mode: json?["mode"].string ?? "",
        modes: (json?["modes"].objects ?? []).map {
            SignInMode(
                value: $0["value"].string,
                title: $0["title"].string,
                desc: $0["desc"].string,
                fields: $0["fields"].strings,
                clears: $0["clears"].strings,
                submit: $0["submit"].string,
                unsaved: $0["unsaved"].string,
                confirm: $0["confirm"].string
            )
        },
        fields: (json?["fields"].objects ?? []).map {
            SignInField(name: $0["name"].string, label: $0["label"].string, value: $0["value"].string, placeholder: $0["placeholder"].string, secret: $0["secret"].bool)
        },
        environment: json?["environment"].string ?? "",
        verdict: parseBadge(json?["verdict"].objectOrNil)
    )
}

private func parseDiagnostics(_ json: JSON?) -> DiagnosticsSettings {
    DiagnosticsSettings(
        status: parseBadge(json?["status"].objectOrNil),
        hint: json?["hint"].string ?? "",
        endpoint: json?["endpoint"].string ?? "",
        token: json?["token"].string ?? "",
        tokenLabel: json?["tokenLabel"].string ?? "",
        streaming: json?["streaming"].bool ?? false,
        canStream: json?["canStream"].bool ?? false,
        state: json?["state"].string ?? "",
        confirm: json?["confirm"].string ?? "",
        offer: parseOffer(json?["offer"])
    )
}

func parseEngineering(_ json: JSON) -> EngineeringView? {
    guard let backend = json["backend"].objectOrNil else { return nil }
    return EngineeringView(
        backend: BackendChoice(
            status: parseBadge(backend["status"].objectOrNil),
            text: backend["text"].string,
            switchTo: backend["switch"].string,
            label: backend["label"].string,
            confirm: backend["confirm"].string,
            success: backend["success"].string
        ),
        publish: parsePublish(json["publish"].objectOrNil),
        diagnostics: parseDiagnostics(json["diagnostics"].objectOrNil)
    )
}

func modeNamed(_ publish: PublishSignIn, _ value: String) -> SignInMode? { publish.modes.first { $0.value == value } }

func signInFields(_ publish: PublishSignIn, _ value: String) -> [SignInField] {
    let names = modeNamed(publish, value)?.fields ?? []
    return publish.fields.filter { names.contains($0.name) }
}

func signInBody(_ publish: PublishSignIn, _ value: String, _ edits: [String: String]) -> [String: String] {
    let clears = modeNamed(publish, value)?.clears ?? []
    return Dictionary(publish.fields.map { ($0.name, clears.contains($0.name) ? "" : edits[$0.name] ?? $0.value) }, uniquingKeysWith: { _, last in last })
}

func signInKey(_ publish: PublishSignIn) -> String { ([publish.mode] + publish.fields.map(\.value)).joined(separator: "|") }
