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
    let filename: String

    var id: Int { index }
    var path: String { "links.linkConfigurations.\(index)" }

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

    enum Editing: Equatable { case hostAndPort, portOnly, serial, logFile, none }

    var editing: Editing {
        switch type {
        case "TypeTcp": return .hostAndPort
        case "TypeLogReplay": return .logFile
        case "TypeUdp": return .portOnly      // UDP binds a local port; there is no single host
        case "TypeSerial": return .serial
        default: return .none
        }
    }

    var displaySummary: String {
        if type == "TypeTcp", host.isEmpty { return "No host set" }
        if type == "TypeLogReplay", filename.isEmpty { return "No log chosen" }
        return summary
    }

    var logFileName: String {
        filename.split(separator: "/").last.map(String.init) ?? filename
    }

    init(index: Int, json: [String: Any]) {
        self.index = index
        name = (json["name"] as? String) ?? ""
        type = (json["linkType"] as? String) ?? ""
        summary = (json["summary"] as? String) ?? ""
        connected = ((json["children"] as? [String]) ?? []).contains("link")
        autoConnect = (json["autoConnect"] as? NSNumber)?.boolValue ?? false
        host = (json["host"] as? String) ?? ""
        port = ((json["port"] ?? json["localPort"]) as? NSNumber)?.intValue ?? 0
        portName = (json["portName"] as? String) ?? ""
        baud = (json["baud"] as? NSNumber)?.intValue ?? 0
        lastError = (json["lastError"] as? String) ?? ""
        filename = (json["filename"] as? String) ?? ""
    }
}

