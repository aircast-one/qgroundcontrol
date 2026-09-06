import Foundation

struct LinkConfig: Identifiable {
    let index: Int
    let name: String
    let type: String
    let summary: String
    let connected: Bool
    let autoConnect: Bool
    let host: String
    let port: Int
    let portName: String
    let baud: Int
    let lastError: String

    var id: Int { index }
    var path: String { "links.linkConfigurations.\(index)" }

    // "TypeTcp" is the C++ enum spelling; operators read "TCP".
    var typeLabel: String {
        let bare = type.hasPrefix("Type") ? String(type.dropFirst(4)) : type
        switch bare.lowercased() {
        case "tcp": return "TCP"
        case "udp": return "UDP"
        case "serial": return "Serial"
        case "bluetooth": return "Bluetooth"
        case "mock": return "Mock"
        case "logreplay": return "Log Replay"
        default: return bare
        }
    }

    enum Editing: Equatable { case hostAndPort, portOnly, serial, none }

    var editing: Editing {
        switch type {
        case "TypeTcp": return .hostAndPort
        case "TypeUdp": return .portOnly      // UDP binds a local port; there is no single host
        case "TypeSerial": return .serial
        default: return .none
        }
    }

    // A TCP link with no host cannot connect, and "TCP · :5760" hides that.
    var displaySummary: String {
        if type == "TypeTcp", host.isEmpty { return "No host set" }
        return summary
    }

    init(index: Int, json: [String: Any]) {
        self.index = index
        name = (json["name"] as? String) ?? ""
        type = (json["linkType"] as? String) ?? ""
        summary = (json["summary"] as? String) ?? ""
        // objectJson lists QObject-valued properties under "children" and omits them
        // from the value map, so a live link shows up as a child, never as a value.
        connected = ((json["children"] as? [String]) ?? []).contains("link")
        autoConnect = (json["autoConnect"] as? NSNumber)?.boolValue ?? false
        host = (json["host"] as? String) ?? ""
        // TCP exposes "port"; UDP exposes "localPort".
        port = ((json["port"] ?? json["localPort"]) as? NSNumber)?.intValue ?? 0
        portName = (json["portName"] as? String) ?? ""
        baud = (json["baud"] as? NSNumber)?.intValue ?? 0
        lastError = (json["lastError"] as? String) ?? ""
    }
}

