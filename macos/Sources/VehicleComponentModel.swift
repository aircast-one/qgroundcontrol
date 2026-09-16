import Foundation

struct VehicleComponentInfo: Identifiable, Equatable {
    let name: String
    let className: String
    let needsAttention: Bool
    let openable: Bool
    let blockedReason: String?
    let known: String?

    var id: String { className.isEmpty ? name : className }

    static let sensorClasses = ["SensorsComponent", "APMSensorsComponent"]

    var isSensors: Bool { VehicleComponentInfo.sensorClasses.contains(className) }

    // ONE WEIGHT FOR ONE FACT. The Summary screen drew a component that needs setup THREE times at
    // once -- the hero button, the "Needs setup" section and the Components list -- and the section
    // used a RED tile while the other two used amber, so the same Radio read as a failure in one
    // row and a warning two rows below it. Amber is the two that agreed and the right weight: a
    // component nobody has configured is not a fault, and the Components list already spends amber
    // on "Reporting a fault", which IS one. Spending red on the lesser of the two inverted them.
    //
    // The rule lives here rather than at the two call sites in VehicleSetupWindow.swift, which
    // swift-checks does not compile, so a future third drawing of the same fact has somewhere to
    // read the answer from instead of picking a colour.
    // The THIRD drawing arrived and read its own answer anyway: the Components list spelled a
    // green tick or an amber exclamation inline from a local `good`, which is this rule AND the
    // sensor fault this property could not see. So one fact had two weights again, the second one
    // in a file swift-checks does not compile. The fault is now an argument rather than a second
    // expression, and the row's WORDS come from the same call, so the mark and the sentence cannot
    // drift apart.
    //
    // needsAttention wins the sentence when both are true. Amber either way, so the weight does not
    // care -- but a component nobody has configured cannot be trusted to be reporting a meaningful
    // fault, and configuring it is the action that clears both.
    func severity(faulted: Bool) -> FlyTelemetry.Level {
        needsAttention || faulted ? .warning : .good
    }

    func statusText(faulted: Bool) -> String {
        needsAttention
            ? VehicleComponentInfo.needsSetup
            : (faulted ? VehicleComponentInfo.reportingFault : "")
    }

    static func statusSymbol(_ weight: FlyTelemetry.Level) -> String {
        weight == .good ? "checkmark.circle.fill" : "exclamationmark.circle.fill"
    }

    // StatusPill spelled this ladder a second time and disagreed on the not-good mark -- an
    // exclamationmark.TRIANGLE where this one uses a CIRCLE -- so the setup window drew two
    // different symbols for one meaning, a component row beside a verdict pill. Nothing recorded
    // the triangle as deliberate, and this is the spelling with assertions behind it, so the pill
    // takes a level from here rather than keeping its own vocabulary.
    static func level(good: Bool) -> FlyTelemetry.Level { good ? .good : .warning }

    static let needsSetup = "Needs setup"
    static let reportingFault = "Reporting a fault"

    // The "Needs setup" section lists only components that need setup, so a fault can never be the
    // reason a row is there -- false is the measured answer for that call site, not a default.
    var severity: FlyTelemetry.Level { severity(faulted: false) }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let name = json["name"] as? String, !name.isEmpty else { return nil }
        self.name = name
        className = (json["className"] as? String) ?? ""
        needsAttention = (json["needsAttention"] as? NSNumber)?.boolValue ?? false
        openable = (json["openable"] as? NSNumber)?.boolValue ?? true
        blockedReason = (json["blockedReason"] as? String).flatMap { $0.isEmpty ? nil : $0 }
        known = (json["known"] as? String).flatMap { $0.isEmpty ? nil : $0 }
    }

    static let blockedWithoutReason = "Disabled"

    var blockedSentence: String? {
        guard !openable else { return nil }
        guard let blockedReason else { return VehicleComponentInfo.blockedWithoutReason }
        return "Disabled while the vehicle is \(blockedReason)"
    }

    static func identity(ofPage page: String) -> String {
        let words = page.split(separator: " ").map(String.init)
        guard let first = words.first else { return "" }
        return ([first.lowercased()] + words.dropFirst()).joined()
    }

    static func page(for component: VehicleComponentInfo, among pages: [String]) -> String? {
        if let known = component.known,
           let named = pages.first(where: { identity(ofPage: $0) == known }) { return named }
        return pages.first { $0 == component.name }
    }

    static func list(_ json: Any?) -> [VehicleComponentInfo] {
        ((json as? [Any]) ?? []).compactMap(VehicleComponentInfo.init)
    }
}

struct VehicleReadiness: Equatable {
    let connected: Bool
    let ready: Bool?
    let headline: String
    let detail: String

    static let unknown = VehicleReadiness(connected: false, ready: nil, headline: "", detail: "")

    var verdict: (text: String, good: Bool)? {
        guard let ready else { return nil }
        return ready ? ("Ready", true) : ("Check", false)
    }

    init(connected: Bool, ready: Bool?, headline: String, detail: String) {
        self.connected = connected
        self.ready = ready
        self.headline = headline
        self.detail = detail
    }

    init(_ json: [String: Any]) {
        connected = (json["connected"] as? NSNumber)?.boolValue ?? false
        ready = (json["ready"] as? NSNumber)?.boolValue
        headline = (json["headline"] as? String) ?? ""
        detail = (json["detail"] as? String) ?? ""
    }
}
