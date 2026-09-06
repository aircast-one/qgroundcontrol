import Foundation
import QGCLinksC

final class LinksStore: ObservableObject, Probeable {
    static let probeID = "links"

    @Published private(set) var links: [LinkConfig] = []
    // View state a test needs to reach lives in the store, not in @State: the probe
    // drives stores, and a locked screen means it cannot click the button instead.
    @Published var adding = false
    @Published var editingIndex: Int?
    @Published private(set) var connectingName = ""
    @Published private(set) var failedName = ""

    private var timer: Timer?

    func reload() {
        let root = Bridge.group("links")
        connectingName = (root["connectingLinkName"] as? String) ?? ""
        failedName = (root["failedLinkName"] as? String) ?? ""

        let model = Bridge.group("links.linkConfigurations")
        links = ((model["elements"] as? [[String: Any]]) ?? [])
            .enumerated()
            .map { LinkConfig(index: $0.offset, json: $0.element) }
    }

    // Connection state changes asynchronously after connect/disconnect, so the list
    // polls while it is on screen rather than lying until the next reopen.
    func startPolling() {
        guard timer == nil else { return }
        reload()
        let timer = Timer(timeInterval: 1.0, repeats: true) { [weak self] _ in self?.reload() }
        RunLoop.main.add(timer, forMode: .common)
        self.timer = timer
    }

    func stopPolling() {
        timer?.invalidate()
        timer = nil
    }

    func connect(_ link: LinkConfig) {
        qgc_links_connect(Int32(link.index))
        reload()
    }

    func disconnect(_ link: LinkConfig) {
        qgc_links_disconnect(Int32(link.index))
        reload()
    }

    func remove(_ link: LinkConfig) {
        qgc_links_remove(Int32(link.index))
        reload()
    }

    @discardableResult
    func create(type: Int, name: String, host: String, port: Int) -> Bool {
        let ok = qgc_links_create(Int32(type), name, host, Int32(port)) == 1
        reload()
        return ok
    }

    // LinkManager exposes the type list; index is the LinkType enum value.
    var linkTypes: [String] {
        (Bridge.group("links")["linkTypeStrings"] as? [String]) ?? []
    }

    func setAutoConnect(_ link: LinkConfig, _ enabled: Bool) {
        Bridge.set("\(link.path).autoConnect", enabled)
        reload()
    }

    func rename(_ link: LinkConfig, to name: String) {
        guard !name.trimmingCharacters(in: .whitespaces).isEmpty else { return }
        Bridge.set("\(link.path).name", name)
        reload()
    }

    func setHost(_ link: LinkConfig, _ host: String) {
        Bridge.set("\(link.path).host", host)
        reload()
    }

    func setPort(_ link: LinkConfig, _ port: Int) {
        Bridge.set("\(link.path).\(link.type == "TypeUdp" ? "localPort" : "port")", port)
        reload()
    }

    func setPortName(_ link: LinkConfig, _ name: String) {
        Bridge.set("\(link.path).portName", name)
        reload()
    }

    func setBaud(_ link: LinkConfig, _ baud: Int) {
        Bridge.set("\(link.path).baud", baud)
        reload()
    }

    var serialPorts: [(label: String, device: String)] {
        let root = Bridge.group("links")
        let labels = (root["serialPortStrings"] as? [String]) ?? []
        let devices = (root["serialPorts"] as? [String]) ?? []
        return zip(labels, devices).map { ($0, $1) }
    }

    var baudRates: [Int] {
        ((Bridge.group("links")["serialBaudRates"] as? [String]) ?? []).compactMap(Int.init)
    }
}

extension LinksStore {
    func probeState() -> [String: Any] {
        ["connecting": connectingName, "failed": failedName,
         "adding": adding, "editing": editingIndex ?? -1,
         "linkTypes": linkTypes, "serialPorts": serialPorts.map(\.label),
         "links": links.map { ["index": $0.index, "name": $0.name, "type": $0.typeLabel,
                               "summary": $0.summary, "connected": $0.connected,
                               "autoConnect": $0.autoConnect, "lastError": $0.lastError] }]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        reload()
        let index = Int(args["index"] ?? "") ?? -1
        let link = links.indices.contains(index) ? links[index] : nil

        switch action {
        case "connect":
            guard let link else { return ["ok": false, "error": "no link at index \(index)"] }
            connect(link)
        case "disconnect":
            guard let link else { return ["ok": false, "error": "no link at index \(index)"] }
            disconnect(link)
        case "remove":
            guard let link else { return ["ok": false, "error": "no link at index \(index)"] }
            remove(link)
        case "create":
            guard create(type: Int(args["type"] ?? "") ?? 2,
                         name: args["name"] ?? "",
                         host: args["host"] ?? "",
                         port: Int(args["port"] ?? "") ?? 0) else {
                return ["ok": false, "error": "could not create link"]
            }
        case "add":
            adding = args["open"] != "false"
        case "edit":
            editingIndex = Int(args["index"] ?? "")
        case "reload":
            break
        default:
            return ["ok": false, "error": "unknown action \(action)"]
        }
        return ["ok": true, "state": probeState()]
    }
}
