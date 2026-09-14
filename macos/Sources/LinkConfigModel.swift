import Foundation

struct LinkConfig: Identifiable, Equatable {
    enum Editing {
        case hostAndPort
        case portOnly
        case serial
        case logFile
        case none
        case unknown

        init(_ reported: String?) {
            switch reported {
            case "hostAndPort": self = .hostAndPort
            case "portOnly": self = .portOnly
            case "serial": self = .serial
            case "logFile": self = .logFile
            case "none": self = .none
            default: self = .unknown
            }
        }
    }

    let index: Int
    let path: String
    let name: String
    let type: String
    let typeLabel: String
    let editing: Editing
    let displaySummary: String
    let statusLine: String
    let connected: Bool
    let heardVehicle: Bool
    let autoConnect: Bool
    let host: String
    let port: Int?
    let portName: String
    let baud: Int?
    let filename: String
    let logFileName: String
    let lastError: String
    let errorRemedy: String

    var id: Int { index }

    var portText: String { port.map(String.init) ?? "" }

    var baudText: String { baud.map(String.init) ?? "" }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let index = (json["index"] as? NSNumber)?.intValue,
              let type = json["type"] as? String else { return nil }
        self.index = index
        self.type = type
        editing = Editing(json["editing"] as? String)
        path = (json["path"] as? String) ?? ""
        name = (json["name"] as? String) ?? ""
        typeLabel = (json["typeLabel"] as? String) ?? ""
        displaySummary = (json["displaySummary"] as? String) ?? ""
        statusLine = (json["statusLine"] as? String) ?? ""
        connected = (json["connected"] as? NSNumber)?.boolValue ?? false
        heardVehicle = (json["heardVehicle"] as? NSNumber)?.boolValue ?? false
        autoConnect = (json["autoConnect"] as? NSNumber)?.boolValue ?? false
        host = (json["host"] as? String) ?? ""
        port = (json["port"] as? NSNumber)?.intValue
        portName = (json["portName"] as? String) ?? ""
        baud = (json["baud"] as? NSNumber)?.intValue
        filename = (json["filename"] as? String) ?? ""
        logFileName = (json["logFileName"] as? String) ?? ""
        lastError = (json["lastError"] as? String) ?? ""
        errorRemedy = (json["errorRemedy"] as? String) ?? ""
    }

    enum Health {
        case closed
        case waiting
        case heard
    }

    var health: Health {
        guard connected else { return .closed }
        return heardVehicle ? .heard : .waiting
    }

    static func list(_ json: Any?) -> [LinkConfig] {
        ((json as? [Any]) ?? []).compactMap(LinkConfig.init)
    }
}

enum LinkTypes {
    static let serial = "serial"

    static func isSerial(_ ids: [String], at index: Int) -> Bool {
        ids.indices.contains(index) && ids[index] == serial
    }

    // The Add Link form reuses one field for two different quantities: a TCP or UDP link puts a
    // PORT there, a serial link puts a BAUD RATE. Validating both as a port refused every rate
    // above the port ceiling -- the form offers 76800, 115200, 230400, 460800, 500000 and 921600,
    // so a serial link at the standard 115200 could not be created at all, and the refusal named
    // a Port the operator was never shown.
    static func accepts(_ entry: String, serial: Bool) -> Bool {
        guard let value = Int(entry) else { return false }
        return serial ? value > 0 : (1...65535).contains(value)
    }

    static func refusal(_ entry: String, serial: Bool) -> String? {
        guard !accepts(entry, serial: serial) else { return nil }
        return serial ? "Choose a baud rate." : "Port must be between 1 and 65535."
    }
}

// A failed link was telling the operator what went wrong and then offering them Connect, which for
// three of TCPLink's cases -- no address, host not found, nothing listening on the port -- is an
// invitation to retry a configuration that cannot succeed until it is changed. QGC's own
// MainStatusIndicatorOfflinePage branches on this and sends them to edit the address instead.
extension LinkConfig {
    var needsAddressEdit: Bool { !lastError.isEmpty && errorRemedy == "editAddress" }

    // Only the editAddress case earns a second line. "retry" is what the Connect button already
    // says, and a sentence telling someone to press the button in front of them is the noise that
    // teaches operators to stop reading this row.
    var remedySentence: String {
        needsAddressEdit ? "Retrying will not help until the address is changed." : ""
    }
}

// The core validates a link's type, host and port together and says what is wrong. This head was
// doing it inline and silently: an out-of-range port fell through `if let port = Int($0)` and
// nothing happened -- no write, no sentence, no sign the keystroke was rejected. An empty host on
// a TCP link was accepted outright, which is the exact state that later produces the editAddress
// remedy, so the editor was creating the failure the row exists to explain.
struct LinkFormCheck: Equatable {
    let valid: Bool
    let error: String
    let name: String

    static let unencodable = LinkFormCheck(
        valid: false,
        error: "An address containing a comma or a bracket cannot be checked yet.",
        name: "")

    init(valid: Bool, error: String, name: String) {
        self.valid = valid
        self.error = error
        self.name = name
    }

    init?(_ json: [String: Any]) {
        guard json["class"] as? String == "LinkForm" else { return nil }
        valid = (json["valid"] as? NSNumber)?.boolValue ?? false
        error = (json["error"] as? String) ?? ""
        name = (json["name"] as? String) ?? ""
    }

    // view.linkForm takes its three arguments inside the view name, and view::split cuts on commas
    // at paren depth zero. A host is free text, so unlike the tlog path this is something an
    // operator can type by accident -- and an address arriving truncated at a comma would be
    // validated as a different address and answered confidently about the wrong one.
    static func encodable(_ parts: String...) -> Bool {
        parts.allSatisfy { !$0.contains(",") && !$0.contains("(") && !$0.contains(")") }
    }
}
