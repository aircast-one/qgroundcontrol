import AppKit
import Foundation
import QGCLinksC
import UniformTypeIdentifiers

final class LinksStore: ObservableObject, Probeable, WriteReporting {
    static let probeID = "links"

    @Published private(set) var links: [LinkConfig] = []
    @Published private(set) var linkTypes: [String] = []
    @Published private(set) var linkTypeIds: [String] = []
    @Published private(set) var baudRates: [Int] = []
    @Published var adding = false
    @Published var editingIndex: Int?
    @Published private(set) var connectingName = ""
    @Published private(set) var failedName = ""
    @Published var writeFailure: String?

    private var timer: Timer?

    func reload() {
        let root = Bridge.group("links")
        let connecting = (root["connectingLinkName"] as? String) ?? ""
        if connecting != connectingName { connectingName = connecting }
        let failed = (root["failedLinkName"] as? String) ?? ""
        if failed != failedName { failedName = failed }

        let view = Bridge.group("view.links")
        let read = LinkConfig.list(view["configured"])
        if read != links { links = read }
        let types = ((view["linkTypes"] as? [Any]) ?? []).compactMap { $0 as? String }
        if types != linkTypes { linkTypes = types }
        let ids = ((view["linkTypeIds"] as? [Any]) ?? []).compactMap { $0 as? String }
        if ids != linkTypeIds { linkTypeIds = ids }
        let rates = ((view["baudRates"] as? [Any]) ?? []).compactMap { ($0 as? NSNumber)?.intValue }
        if rates != baudRates { baudRates = rates }
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
}

extension LinksStore {
    func probeState() -> [String: Any] {
        ["connecting": connectingName, "failed": failedName,
         "writeFailure": writeFailure ?? "",
         "adding": adding, "editing": editingIndex ?? -1,
         "linkTypes": linkTypes, "serialPorts": serialPorts.map(\.label),
         "links": links.map { ["index": $0.index, "name": $0.name, "type": $0.typeLabel,
                               "summary": $0.displaySummary, "connected": $0.connected,
                               "heardVehicle": $0.heardVehicle, "statusLine": $0.statusLine,
                               "health": "\($0.health)",
                               "autoConnect": $0.autoConnect, "lastError": $0.lastError] }]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        reload()
        let index = Int(args["index"] ?? "") ?? -1
        let link = links.first { $0.index == index }

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
