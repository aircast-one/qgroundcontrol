import Foundation

final class ParametersStore: ObservableObject, Probeable, WriteReporting {
    static let probeID = "parameters"

    @Published private(set) var parameters: [Parameter] = []
    @Published private(set) var loading = false
    @Published private(set) var status = ""
    @Published var writeFailure: String?
    @Published private(set) var connected = false
    @Published var search = "" { didSet { refilter() } }
    @Published var group = "" { didSet { refilter() } }
    @Published private(set) var visible: [Parameter] = []

    private let componentId = 1
    // Cleared by EVERY parameter write, not only by writes made through a setup control.
    // 3c96e2ea2 made a section's CONTENT depend on a parameter's VALUE: a block whose first
    // parameter is a *_MONITOR serves only that parameter while it reads zero, because ArduPilot
    // does not populate the rest until a monitor is set. Before that, a section's shape was fixed
    // and a cache surviving writes cost nothing.
    private var setupCache: [String: [SettingsSection]] = [:]
    private var cachedFor: Int?

    var groups: [String] {
        Array(Set(parameters.map(\.group))).sorted()
    }

    @Published private(set) var parameterStatus = ""

    func load() {
        guard !loading else { return }
        // connected and the load state come from ONE read. They were two -- Bridge.group("vehicle")
        // for the flag and Bridge.group("vehicle.parameterManager") for the gate, three lines apart
        // so they looked like one operation -- and a disconnect between them reads a dead vehicle's
        // manager beside a fresh connected. view.setup composes both, and answers noVehicle in that
        // gap rather than a stale unanswered.
        let setup = Bridge.group("view.setup")
        connected = (setup["connected"] as? NSNumber)?.boolValue ?? false
        let identity = (Bridge.group("vehicle")["id"] as? NSNumber)?.intValue
        if !SettingsSection.cacheSurvives(vehicle: cachedFor, now: identity) { setupCache.removeAll() }
        cachedFor = identity
        parameterStatus = VehicleSetupText.parameters(
            reason: (setup["parametersReason"] as? String) ?? "",
            served: (setup["parametersText"] as? String) ?? "")
        guard (setup["parametersReady"] as? NSNumber)?.boolValue == true else {
            status = parameterStatus
            parameters = []
            refilter()
            return
        }

        loading = true
        status = ""

        let names = (Bridge.invoke("vehicle.parameterManager.parameterNames",
                                   [componentId])["result"] as? [String]) ?? []
        let loaded = names.compactMap { name -> Parameter? in
            let json = Bridge.group("vehicle.parameterManager.getParameter(\(componentId),\(name))")
            guard json["kind"] as? String == "fact" else { return nil }
            return Parameter(name: name, componentId: componentId, json: json)
        }

        parameters = loaded
        loading = false
        status = ParameterLoad.status(named: names.count, loaded: loaded.count)
        refilter()
    }


    func parameter(named name: String) -> Parameter? {
        parameters.first { $0.name == name }
    }

    func sections(of page: String) -> [SettingsSection] {
        if let cached = setupCache[page] { return cached }
        let read = SettingsSection.list(Bridge.group("view.setup(\(page))")["sections"])
            .filter { !$0.controls.isEmpty }
        if SettingsSection.worthRemembering(read) { setupCache[page] = read }
        return read
    }

    func writeControl(_ control: SettingsControl, _ value: String) {
        if let refused = control.refusal(value) {
            writeFailure = refused
            return
        }
        guard write(control.path, Double(value) ?? value, control.label) else { return }
        setupCache.removeAll()
        objectWillChange.send()
    }

    func refilter() {
        let needle = search.trimmingCharacters(in: .whitespaces).lowercased()
        visible = parameters.filter { parameter in
            (group.isEmpty || parameter.group == group)
                && (needle.isEmpty
                    || parameter.name.lowercased().contains(needle)
                    || parameter.description.lowercased().contains(needle))
        }
    }

    func write(_ parameter: Parameter, _ value: String) {
        if let refused = parameter.refusal(value) {
            writeFailure = refused
            return
        }
        guard write(parameter.path, Double(value) ?? value, parameter.name) else { return }
        setupCache.removeAll()
        let json = Bridge.group(parameter.path)
        guard json["kind"] as? String == "fact" else { return }
        let updated = Parameter(name: parameter.name, componentId: parameter.componentId, json: json)
        if let index = parameters.firstIndex(where: { $0.id == updated.id }) {
            parameters[index] = updated
        }
        refilter()
    }

    func probeState() -> [String: Any] {
        ["writeFailure": writeFailure ?? "", "connected": connected,
         "count": parameters.count, "visible": visible.count, "loading": loading,
         "search": search, "group": group, "status": status,
         "groups": groups.prefix(12).map { $0 },
         "sample": visible.prefix(5).map { ["name": $0.name, "value": $0.value, "units": $0.units] }]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        switch action {
        case "failWrite":
            guard let sample = parameters.first else {
                return ["ok": false, "error": "no parameters loaded"]
            }
            writeFailure = WriteReport.failure(sample.name)
        case "load":
            load()
        case "search": search = args["text"] ?? ""
        case "group": group = args["name"] ?? ""
        case "set":
            guard let name = args["name"], let value = args["value"] else {
                return ["ok": false, "error": "set needs name and value"]
            }
            guard let parameter = parameters.first(where: { $0.name == name }) else {
                return ["ok": false, "error": "no parameter \(name)"]
            }
            write(parameter, value)
        default:
            return ["ok": false, "error": "unknown action \(action)"]
        }
        return ["ok": true, "state": probeState()]
    }
}
