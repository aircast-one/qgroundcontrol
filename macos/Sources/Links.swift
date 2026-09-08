import AppKit
import Foundation
import QGCLinksC
import UniformTypeIdentifiers

final class LinksStore: ObservableObject, Probeable, WriteReporting {
    static let probeID = "links"

    @Published private(set) var links: [LinkConfig] = []
    @Published var adding = false
    @Published var editingIndex: Int?
    @Published private(set) var connectingName = ""
    @Published private(set) var failedName = ""
    @Published var writeFailure: String?

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
        Bridge.invoke("links.createConnectedLink", ["@\(link.path)"])
        reload()
    }

    func disconnect(_ link: LinkConfig) {
        Bridge.invoke("\(link.path).link.disconnect")
        reload()
    }

    func remove(_ link: LinkConfig) {
        Bridge.invoke("links.removeConfiguration", ["@\(link.path)"])
        reload()
    }

    @discardableResult
    func create(type: Int, name: String, host: String, port: Int) -> Bool {
        let ok = qgc_links_create(Int32(type), name, host, Int32(port)) == 1
        reload()
        return ok
    }

    var linkTypes: [String] {
        (Bridge.group("links")["linkTypeStrings"] as? [String]) ?? []
    }

    func setAutoConnect(_ link: LinkConfig, _ enabled: Bool) {
        write("\(link.path).autoConnect", enabled, "whether this link connects on its own")
        reload()
    }

    func rename(_ link: LinkConfig, to name: String) {
        guard !name.trimmingCharacters(in: .whitespaces).isEmpty else { return }
        write("\(link.path).name", name, "the link name")
        reload()
    }

    func chooseLogFile(_ link: LinkConfig) {
        let panel = NSOpenPanel()
        panel.title = "Select a telemetry log to replay"
        panel.canChooseDirectories = false
        panel.allowedContentTypes = ["tlog", "log"].compactMap { UTType(filenameExtension: $0) }
        panel.allowsOtherFileTypes = true
        guard panel.runModal() == .OK, let url = panel.url else { return }
        setLogFile(link, url.path)
    }

    func setLogFile(_ link: LinkConfig, _ path: String) {
        guard !path.isEmpty else { return }
        write("\(link.path).filename", path, "the log file")
        reload()
    }

    func setHost(_ link: LinkConfig, _ host: String) {
        write("\(link.path).host", host, "the host")
        reload()
    }

    func setPort(_ link: LinkConfig, _ port: Int) {
        write("\(link.path).\(link.type == "TypeUdp" ? "localPort" : "port")", port, "the port")
        reload()
    }

    func setPortName(_ link: LinkConfig, _ name: String) {
        write("\(link.path).portName", name, "the serial port")
        reload()
    }

    func setBaud(_ link: LinkConfig, _ baud: Int) {
        write("\(link.path).baud", baud, "the baud rate")
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
         "writeFailure": writeFailure ?? "",
         "adding": adding, "editing": editingIndex ?? -1,
         "linkTypes": linkTypes, "serialPorts": serialPorts.map(\.label),
         "links": links.map { ["index": $0.index, "name": $0.name, "type": $0.typeLabel,
                               "summary": $0.displaySummary, "connected": $0.connected,
                               "autoConnect": $0.autoConnect, "lastError": $0.lastError] }]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        reload()
        let index = Int(args["index"] ?? "") ?? -1
        let link = links.indices.contains(index) ? links[index] : nil

        switch action {
        case "failWrite":
            write(args["path"] ?? "links.linkConfigurations.0.linkType",
                  args["value"] ?? "x", args["what"] ?? "the link type")
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
