import Foundation

final class ParametersStore: ObservableObject, Probeable {
    static let probeID = "parameters"

    @Published private(set) var parameters: [Parameter] = []
    @Published private(set) var loading = false
    @Published private(set) var status = ""
    @Published var search = "" { didSet { refilter() } }
    @Published var group = "" { didSet { refilter() } }
    @Published private(set) var visible: [Parameter] = []

    private let componentId = 1

    var groups: [String] {
        Array(Set(parameters.map(\.group))).sorted()
    }

    func load() {
        guard !loading else { return }
        let manager = Bridge.group("vehicle.parameterManager")
        guard (manager["parametersReady"] as? NSNumber)?.boolValue == true else {
            status = "Waiting for parameters from the vehicle…"
            parameters = []
            refilter()
            return
        }

        loading = true
        status = ""

        // ~1400 bridge reads. Each one marshals to the Qt thread on its own, so this
        // belongs off the main thread: run inline and the "loading" state never gets a
        // chance to render, which is the same as lying about it.
        let component = componentId
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            let names = (Bridge.invoke("vehicle.parameterManager.parameterNames",
                                       [component])["result"] as? [String]) ?? []
            let loaded = names.compactMap { name -> Parameter? in
                let json = Bridge.group("vehicle.parameterManager.getParameter(\(component),\(name))")
                guard json["kind"] as? String == "fact" else { return nil }
                return Parameter(name: name, componentId: component, json: json)
            }
            DispatchQueue.main.async {
                guard let self else { return }
                self.parameters = loaded
                self.loading = false
                self.status = loaded.isEmpty ? "This vehicle reported no parameters." : ""
                self.refilter()
            }
        }
    }

    // The probe drives this synchronously, so it needs to wait for the async load.
    private func waitForLoad() {
        for _ in 0..<200 where loading {
            RunLoop.current.run(until: Date().addingTimeInterval(0.05))
        }
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

    // Writes go to the vehicle immediately, which is how QGC's parameter editor
    // behaves; the value is read back because a Fact clamps out-of-range input.
    func write(_ parameter: Parameter, _ value: String) {
        Bridge.set(parameter.path, Double(value) ?? value)
        let json = Bridge.group(parameter.path)
        guard json["kind"] as? String == "fact" else { return }
        let updated = Parameter(name: parameter.name, componentId: parameter.componentId, json: json)
        if let index = parameters.firstIndex(where: { $0.id == updated.id }) {
            parameters[index] = updated
        }
        refilter()
    }

    func probeState() -> [String: Any] {
        ["count": parameters.count, "visible": visible.count, "loading": loading,
         "search": search, "group": group, "status": status,
         "groups": groups.prefix(12).map { $0 },
         "sample": visible.prefix(5).map { ["name": $0.name, "value": $0.value, "units": $0.units] }]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        switch action {
        case "load":
            load()
            waitForLoad()
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
