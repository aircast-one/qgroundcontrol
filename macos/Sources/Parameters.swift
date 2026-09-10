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
    private var setupCache: [String: [SettingsSection]] = [:]

    var groups: [String] {
        Array(Set(parameters.map(\.group))).sorted()
    }

    func load() {
        guard !loading else { return }
        connected = Bridge.group("vehicle")["kind"] as? String == "object"
        let manager = Bridge.group("vehicle.parameterManager")
        guard (manager["parametersReady"] as? NSNumber)?.boolValue == true else {
            status = VehicleSetupText.waiting(connected: connected, for: "parameters")
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
        status = loaded.isEmpty ? "This vehicle reported no parameters." : ""
        refilter()
    }


    func parameter(named name: String) -> Parameter? {
        parameters.first { $0.name == name }
    }

    func sections(of page: String) -> [SettingsSection] {
        if let cached = setupCache[page] { return cached }
        let read = SettingsSection.list(Bridge.group("view.setup(\(page))")["sections"])
            .filter { !$0.controls.isEmpty }
        setupCache[page] = read
        return read
    }

    func writeControl(_ control: SettingsControl, _ value: String) {
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
        guard write(parameter.path, Double(value) ?? value, parameter.name) else { return }
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
