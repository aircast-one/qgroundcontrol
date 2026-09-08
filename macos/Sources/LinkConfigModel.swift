import Foundation

struct LinkConfig: Identifiable {
    let index: Int
    let name: String
    let type: String
    let title: String
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

    static let tcp = "TcpSettings.qml"
    static let udp = "UdpSettings.qml"
    static let serial = "SerialSettings.qml"
    static let bluetooth = "BluetoothSettings.qml"
    static let logReplay = "LogReplaySettings.qml"

    var typeLabel: String {
        switch type {
        case LinkConfig.tcp: return "TCP"
        case LinkConfig.udp: return "UDP"
        case LinkConfig.serial: return "Serial"
        case LinkConfig.bluetooth: return "Bluetooth"
        case LinkConfig.logReplay: return "Log Replay"
        default: return LinkConfig.strip(title)
        }
    }

    static func strip(_ title: String) -> String {
        let dropped = title.replacingOccurrences(of: " Link Settings", with: "")
        return dropped == title
            ? title.replacingOccurrences(of: " Settings", with: "")
            : dropped
    }

    enum Editing: Equatable { case hostAndPort, portOnly, serial, logFile, none }

    var editing: Editing {
        switch type {
        case LinkConfig.tcp: return .hostAndPort
        case LinkConfig.logReplay: return .logFile
        case LinkConfig.udp: return .portOnly
        case LinkConfig.serial: return .serial
        default: return .none
        }
    }

    var displaySummary: String {
        if type == LinkConfig.tcp, host.isEmpty { return "No host set" }
        if type == LinkConfig.logReplay, filename.isEmpty { return "No log chosen" }
        return summary
    }

    var logFileName: String {
        filename.split(separator: "/").last.map(String.init) ?? filename
    }

    init(index: Int, json: [String: Any]) {
        self.index = index
        name = (json["name"] as? String) ?? ""
        type = (json["settingsURL"] as? String) ?? ""
        title = (json["settingsTitle"] as? String) ?? ""
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

