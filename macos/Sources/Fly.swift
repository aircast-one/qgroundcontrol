import Foundation

final class FlyStore: ObservableObject, Probeable {
    static let probeID = "fly"

    @Published private(set) var telemetry = FlyTelemetry()
    @Published private(set) var connected = false
    @Published private(set) var position: VehicleMarker?
    @Published private(set) var messages: [VehicleMessage] = []
    @Published private(set) var unhealthyBits: Int?
    @Published private(set) var warning = VehicleWarning.none
    @Published private(set) var airframe = PreflightAirframe.generic
    @Published private(set) var audioMuted = false
    @Published private(set) var batteries: [[DetailRow]] = []
    @Published private(set) var gpsDetail: [DetailRow] = []
    @Published private(set) var linkDetail: [DetailRow] = []
    @Published var expanded: Set<String> = []
    @Published private(set) var modes: [FlightModeChoice] = []
    @Published private(set) var requestedMode = ""
    @Published var showingModes = false
    @Published var showingAdvancedModes = false
    @Published private(set) var confirmingMode = ""
    private var rtlMode = ""
    private var landMode = ""
    @Published private(set) var ticked: Set<String> = []
    @Published var showingChecklist = false

    private var poll: Timer?

    func start() {
        refresh()
        poll?.invalidate()
        poll = Timer.scheduledTimer(withTimeInterval: 0.5, repeats: true) { [weak self] _ in
            self?.refresh()
        }
    }

    func stop() {
        poll?.invalidate()
        poll = nil
    }

    func refresh() {
        let muted = (Bridge.group("settings.appSettings.audioMuted")["value"] as? NSNumber)?.boolValue ?? false
        if muted != audioMuted { audioMuted = muted }

        let vehicle = Bridge.group("vehicle")
        guard vehicle["kind"] as? String == "object" else {
            if connected { connected = false }
            if telemetry != FlyTelemetry() { telemetry = FlyTelemetry() }
            if position != nil { position = nil }
            if !messages.isEmpty { messages = [] }
            if unhealthyBits != nil { unhealthyBits = nil }
            if warning != .none { warning = .none }
            if airframe != .generic { airframe = .generic }
            if !batteries.isEmpty { batteries = [] }
            if !gpsDetail.isEmpty { gpsDetail = [] }
            if !linkDetail.isEmpty { linkDetail = [] }
            if !modes.isEmpty { modes = [] }
            return
        }

        var reading = FlyTelemetry()
        reading.mode = (vehicle["flightMode"] as? String) ?? ""
        reading.armed = (vehicle["armed"] as? NSNumber)?.boolValue ?? false
        reading.flying = (vehicle["flying"] as? NSNumber)?.boolValue ?? false

        let facts = FlyStore.facts(vehicle)
        reading.altitude = facts["altitudeRelative"]
        reading.groundSpeed = facts["groundSpeed"]
        reading.climbRate = facts["climbRate"]
        reading.heading = facts["heading"]

        let gpsGroup = Bridge.group("vehicle.gps")
        let readGps = FlyDetail.gps(FactReading.from((gpsGroup["facts"] as? [Any]) ?? []))
        if readGps != gpsDetail { gpsDetail = readGps }

        let gps = FlyStore.facts(gpsGroup)
        reading.satellites = gps["count"].map { Int($0) }
        reading.gpsLock = gps["lock"].map { Int($0) }

        let packs = (Bridge.group("vehicle.batteries")["elements"] as? [[String: Any]]) ?? []
        let readPacks = packs.map { FlyDetail.battery(FactReading.from(($0["facts"] as? [Any]) ?? [])) }
        if readPacks != batteries { batteries = readPacks }

        let readLink = FlyDetail.link(
            rcRSSI: (vehicle["rcRSSI"] as? NSNumber)?.intValue,
            localRSSI: (vehicle["telemetryLRSSI"] as? NSNumber)?.intValue,
            remoteRSSI: (vehicle["telemetryRRSSI"] as? NSNumber)?.intValue)
        if readLink != linkDetail { linkDetail = readLink }

        let battery = packs.first
        let batteryFacts = battery.map(FlyStore.facts) ?? [:]
        reading.batteryPercent = batteryFacts["percentRemaining"]
        reading.batteryVolts = batteryFacts["voltage"]

        let coordinate = vehicle["coordinate"] as? [String: Any]
        let placed = VehicleMarker(
            latitude: (coordinate?["latitude"] as? NSNumber)?.doubleValue,
            longitude: (coordinate?["longitude"] as? NSNumber)?.doubleValue,
            heading: reading.heading)

        if !connected { connected = true }
        if reading != telemetry { telemetry = reading }
        if placed != position { position = placed }

        let heard = VehicleMessage.parse((vehicle["formattedMessages"] as? String) ?? "")
        if heard != messages { messages = heard }

        let readModes = FlightModes.choices(
            all: (vehicle["flightModes"] as? [String]) ?? [],
            advanced: (vehicle["advancedFlightModes"] as? [String]) ?? [],
            current: reading.mode)
        if readModes != modes { modes = readModes }
        rtlMode = (vehicle["rtlFlightMode"] as? String) ?? ""
        landMode = (vehicle["landFlightMode"] as? String) ?? ""
        if !requestedMode.isEmpty, requestedMode == reading.mode { requestedMode = "" }

        let flown = PreflightAirframe.of(
            multiRotor: FlyStore.flag(vehicle, "multiRotor"), vtol: FlyStore.flag(vehicle, "vtol"),
            rover: FlyStore.flag(vehicle, "rover"), sub: FlyStore.flag(vehicle, "sub"),
            fixedWing: FlyStore.flag(vehicle, "fixedWing"))
        if flown != airframe { airframe = flown }

        let bits = (vehicle["sensorsUnhealthyBits"] as? NSNumber)?.intValue
        if bits != unhealthyBits { unhealthyBits = bits }

        let health = Bridge.group("vehicle.healthAndArmingCheckReport")
        let assessed = VehicleWarning.assess(
            connected: true,
            requiresGpsFix: (vehicle["requiresGpsFix"] as? NSNumber)?.boolValue ?? false,
            hasCoordinate: placed != nil,
            armed: reading.armed,
            prearmError: (vehicle["prearmError"] as? String) ?? "",
            healthReportSupported: (health["supported"] as? NSNumber)?.boolValue ?? false)
        if assessed != warning { warning = assessed }
    }

    private static func flag(_ vehicle: [String: Any], _ name: String) -> Bool {
        (vehicle[name] as? NSNumber)?.boolValue ?? false
    }

    private static func facts(_ object: [String: Any]) -> [String: Double] {
        ((object["facts"] as? [[String: Any]]) ?? []).reduce(into: [String: Double]()) { values, fact in
            guard let name = fact["name"] as? String,
                  let value = (fact["value"] as? NSNumber)?.doubleValue, value.isFinite else { return }
            values[name] = value
        }
    }

    var latestMessages: [VehicleMessage] { Array(messages.prefix(FlyStore.messageLimit)) }

    var checklist: [PreflightGroup] {
        Preflight.groups(airframe: airframe, lock: telemetry.gpsLock, satellites: telemetry.satellites,
                         batteryPercent: telemetry.batteryPercent, unhealthyBits: unhealthyBits,
                         audioMuted: audioMuted)
    }

    func toggle(_ check: PreflightCheck) {
        guard !check.blocked else { return }
        ticked = ticked.contains(check.name)
            ? ticked.subtracting([check.name])
            : ticked.union([check.name])
    }

    func resetChecklist() {
        ticked = []
    }

    func request(_ mode: FlightModeChoice) {
        guard !mode.current, modes.contains(mode) else { return }
        guard !FlightModes.needsConfirming(mode.name, flying: telemetry.flying,
                                           rtlMode: rtlMode, landMode: landMode)
        else {
            confirmingMode = mode.name
            return
        }
        send(mode.name)
    }

    func confirmMode() {
        guard !confirmingMode.isEmpty else { return }
        send(confirmingMode)
    }

    func cancelMode() {
        confirmingMode = ""
    }

    private func send(_ name: String) {
        requestedMode = name
        confirmingMode = ""
        _ = Bridge.set("vehicle.flightMode", name)
        showingModes = false
        refresh()
    }

    static let messageLimit = 6

    func probeState() -> [String: Any] {
        ["connected": connected, "mode": telemetry.mode, "state": telemetry.stateText,
         "altitude": FlyTelemetry.metres(telemetry.altitude),
         "groundSpeed": FlyTelemetry.speed(telemetry.groundSpeed),
         "heading": FlyTelemetry.degrees(telemetry.heading),
         "battery": telemetry.batteryText, "gps": telemetry.gpsText,
         "placed": position != nil,
         "worstMessage": VehicleMessage.worst(latestMessages).rawValue,
         "warnings": warning.lines,
         "checklistOpen": showingChecklist,
         "airframe": airframe.rawValue,
         "modes": modes.map(\.name),
         "everydayModes": FlightModes.everyday(modes).map(\.name),
         "foldedModes": FlightModes.folded(modes).map(\.name),
         "requestedMode": requestedMode,
         "modesOpen": showingModes,
         "confirmingMode": confirmingMode,
         "expanded": Array(expanded).sorted(),
         "batteryDetail": batteries.map { pack in pack.map { "\($0.label): \($0.value)" } },
         "gpsDetail": gpsDetail.map { "\($0.label): \($0.value)" },
         "linkDetail": linkDetail.map { "\($0.label): \($0.value)" },
         "checklistNames": checklist.flatMap(\.checks).map(\.name),
         "checklistProgress": Preflight.progress(checklist, ticked: ticked),
         "checklistReady": Preflight.ready(checklist, ticked: ticked),
         "checklistBlocked": checklist.flatMap(\.checks).filter(\.blocked).map(\.name),
         "messages": latestMessages.map { ["time": $0.time, "text": $0.text, "level": $0.level.rawValue] }]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        switch action {
        case "refresh": refresh()
        case "checklist": showingChecklist = args["open"] != "0"
        case "resetChecklist": resetChecklist()
        case "modes":
            showingModes = args["open"] != "0"
            if !showingModes { showingAdvancedModes = false }
        case "confirmMode":
            args["on"] == "0" ? cancelMode() : confirmMode()
        case "moreModes":
            showingAdvancedModes = args["on"] != "0"
        case "expand":
            let row = args["row"] ?? ""
            expanded = args["on"] == "0" ? expanded.subtracting([row]) : expanded.union([row])
        case "tick":
            guard let check = checklist.flatMap(\.checks).first(where: { $0.name == args["check"] ?? "" }) else {
                return ["ok": false, "error": "no check named \(args["check"] ?? "")"]
            }
            guard !check.blocked else {
                return ["ok": false, "error": "\(check.name) is blocked: \(check.reason)"]
            }
            toggle(check)
        default: return ["ok": false, "error": "unknown action \(action)"]
        }
        return ["ok": true, "state": probeState()]
    }
}
