import Foundation

final class FlyStore: ObservableObject, Probeable, WriteReporting {
    @Published var writeFailure: String?
    static let probeID = "fly"

    @Published private(set) var telemetry = FlyTelemetry()
    @Published private(set) var connected = false
    @Published private(set) var position: VehicleMarker?
    @Published private(set) var messages: [VehicleMessage] = []
    @Published private(set) var warnings: [VehicleWarning] = []
    @Published private(set) var armingBlocker: String?
    @Published private(set) var airframe = "Generic"
    @Published private(set) var checklist: [PreflightGroup] = []
    @Published private(set) var batteries: [[DetailRow]] = []
    @Published private(set) var batteryLevels: [FlyTelemetry.Level] = []
    @Published private(set) var gpsDetail: [DetailRow] = []
    @Published private(set) var linkDetail: [DetailRow] = []
    @Published var expanded: Set<String> = []
    @Published private(set) var modes: [FlightModeChoice] = []
    @Published private(set) var canSetMode = false
    @Published private(set) var requestedMode = ""
    @Published var showingModes = false
    @Published var showingAdvancedModes = false
    @Published private(set) var confirmingMode = ""
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

    @Published private(set) var keepCentered = false
    @Published private(set) var terrain = TerrainDownload.none
    @Published private(set) var terrainShowing = false
    private var terrainIdleSince: Date?

    private func readTerrain() {
        let read = TerrainDownload.read(
            (Bridge.group("vehicle.terrain")["facts"] as? [[String: Any]]) ?? [])
        if read != terrain {
            terrainIdleSince = read.busy ? nil : (read.started ? Date() : nil)
            terrain = read
        }
        let showing = TerrainDownload.showing(
            terrain, sinceIdle: terrainIdleSince.map { Date().timeIntervalSince($0) })
        if showing != terrainShowing { terrainShowing = showing }
    }

    func refresh() {
        let centred = (Bridge.group("settings.flyViewSettings.keepMapCenteredOnVehicle")["value"]
            as? NSNumber)?.boolValue ?? false
        if centred != keepCentered { keepCentered = centred }

        readTerrain()
        readChecklist()

        let vehicle = Bridge.group("vehicle")
        guard vehicle["kind"] as? String == "object" else {
            if connected { connected = false }
            if telemetry != FlyTelemetry() { telemetry = FlyTelemetry() }
            if position != nil { position = nil }
            if !messages.isEmpty { messages = [] }
            if !warnings.isEmpty { warnings = [] }
            if armingBlocker != nil { armingBlocker = nil }
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
        let units = FlyStore.units(vehicle)
        reading.distanceUnits = units["altitudeRelative"] ?? "m"
        reading.speedUnits = units["groundSpeed"] ?? "m/s"
        reading.altitude = facts["altitudeRelative"]
        reading.groundSpeed = facts["groundSpeed"]
        reading.climbRate = facts["climbRate"]
        reading.heading = facts["heading"]

        let links = Bridge.group("vehicle.vehicleLinkManager")
        reading.contactLost = (links["communicationLost"] as? NSNumber)?.boolValue ?? false

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

        let batteryView = Bridge.group("view.battery")
        let batteryPacks = (batteryView["packs"] as? [[String: Any]]) ?? []
        let first = batteryPacks.first
        reading.batteryPercent = (first?["percent"] as? NSNumber)?.doubleValue
        reading.batteryVolts = (first?["voltage"] as? NSNumber)?.doubleValue
        reading.batteryLevel = FlyTelemetry.Level(batteryView["level"] as? String)
        reading.batteryText = FlyTelemetry.batteryLine((first?["text"] as? String) ?? "",
                                                       (first?["secondaryText"] as? String) ?? "")
        let readLevels = batteryPacks.map { FlyTelemetry.Level($0["level"] as? String) }
        if readLevels != batteryLevels { batteryLevels = readLevels }

        let coordinate = vehicle["coordinate"] as? [String: Any]
        let placed = VehicleMarker(
            latitude: (coordinate?["latitude"] as? NSNumber)?.doubleValue,
            longitude: (coordinate?["longitude"] as? NSNumber)?.doubleValue,
            heading: reading.heading)

        if !connected { connected = true }
        if reading != telemetry { telemetry = reading }
        if placed != position { position = placed }

        let heard = VehicleMessage.list(Bridge.group("view.messages")["items"])
        if heard != messages { messages = heard }

        let flightModes = Bridge.group("view.flightModes")
        let readModes = FlightModes.list(flightModes["modes"])
        if readModes != modes { modes = readModes }
        let settable = (flightModes["canSet"] as? NSNumber)?.boolValue ?? false
        if settable != canSetMode { canSetMode = settable }
        if !requestedMode.isEmpty, requestedMode == reading.mode { requestedMode = "" }

        let raised = Bridge.group("view.warnings")
        let assessed = VehicleWarning.list(raised["warnings"])
        if assessed != warnings { warnings = assessed }
        let blocker = raised["armingBlocker"] as? String
        if blocker != armingBlocker { armingBlocker = blocker }
    }

    private static func units(_ object: [String: Any]) -> [String: String] {
        ((object["facts"] as? [[String: Any]]) ?? []).reduce(into: [String: String]()) { found, fact in
            guard let name = fact["name"] as? String,
                  let units = fact["units"] as? String, !units.isEmpty else { return }
            found[name] = Units.display(units)
        }
    }

    private static func facts(_ object: [String: Any]) -> [String: Double] {
        ((object["facts"] as? [[String: Any]]) ?? []).reduce(into: [String: Double]()) { values, fact in
            guard let name = fact["name"] as? String,
                  let value = (fact["value"] as? NSNumber)?.doubleValue, value.isFinite else { return }
            values[name] = value
        }
    }

    var latestMessages: [VehicleMessage] { Array(messages.prefix(FlyStore.messageLimit)) }

    // The core answers the checklist with no vehicle connected, and says so in each reason.
    // "Before you power up" is the group an operator works through while nothing is connected
    // yet, so it does not sit behind the guard that clears vehicle telemetry.
    private func readChecklist() {
        let preflight = Bridge.group("view.preflight")
        let flown = (preflight["airframe"] as? String) ?? "Generic"
        if flown != airframe { airframe = flown }
        let read = Preflight.groups(preflight["groups"])
        if read != checklist { checklist = read }
    }

    func toggle(_ check: PreflightCheck) {
        guard check.tickable else { return }
        ticked = ticked.contains(check.name)
            ? ticked.subtracting([check.name])
            : ticked.union([check.name])
    }

    func resetChecklist() {
        ticked = []
    }

    func request(_ mode: FlightModeChoice) {
        guard canSetMode, !mode.current, modes.contains(mode) else { return }
        guard !mode.needsConfirm else {
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
        confirmingMode = ""
        showingModes = false
        if write("vehicle.flightMode", name, "the flight mode") { requestedMode = name }
        refresh()
    }

    static let messageLimit = 6

    func probeState() -> [String: Any] {
        ["writeFailure": writeFailure ?? "",
         "connected": connected, "mode": telemetry.mode, "state": telemetry.stateText,
         "contactLost": telemetry.contactLost, "staleNotice": telemetry.staleNotice,
         "altitude": telemetry.altitudeText,
         "groundSpeed": telemetry.groundSpeedText,
         "keepCentered": keepCentered,
         "terrain": ["loaded": terrain.loaded, "pending": terrain.pending,
                     "text": terrain.text, "percent": terrain.percentText,
                     "showing": terrainShowing],
         "heading": FlyTelemetry.degrees(telemetry.heading),
         "battery": telemetry.batteryText, "gps": telemetry.gpsText,
         "placed": position != nil,
         "worstMessage": VehicleMessage.worst(latestMessages).rawValue,
         "warnings": warnings.map(\.text),
         "warningDetails": warnings.map(\.detail),
         "armingBlocker": armingBlocker ?? "",
         "checklistOpen": showingChecklist,
         "airframe": airframe,
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
         "checklistChecks": checklist.flatMap(\.checks).map {
             ["name": $0.name, "verdict": $0.verdict.rawValue, "symbol": $0.symbol(ticked: ticked),
              "met": $0.met(ticked: ticked), "tickable": $0.tickable, "hint": $0.hint]
         },
         "messages": latestMessages.map {
             ["time": $0.time, "stamp": $0.stamp, "text": $0.text, "level": $0.level.rawValue,
              "id": $0.id]
         }]
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
            guard check.tickable else {
                return ["ok": false,
                        "error": check.blocks ? "\(check.name) is blocked: \(check.reason)"
                                              : "\(check.name) needs no tick: \(check.reason)"]
            }
            toggle(check)
        default: return ["ok": false, "error": "unknown action \(action)"]
        }
        return ["ok": true, "state": probeState()]
    }
}
