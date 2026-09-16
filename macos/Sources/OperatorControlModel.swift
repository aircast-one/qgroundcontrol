import Foundation

struct OperatorControl: Equatable {
    let available: Bool
    let known: Bool
    let inControl: Bool?
    let holderSystemId: Int?
    let takeoverAllowed: Bool?
    let systemManager: Bool?
    let requestAllowed: Bool

    static let none = OperatorControl()

    private init() {
        available = false
        known = false
        inControl = nil
        holderSystemId = nil
        takeoverAllowed = nil
        systemManager = nil
        requestAllowed = false
    }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any], json["kind"] as? String == "object" else {
            return nil
        }
        available = (json["available"] as? NSNumber)?.boolValue ?? false
        known = (json["known"] as? NSNumber)?.boolValue ?? false
        inControl = (json["inControl"] as? NSNumber)?.boolValue
        holderSystemId = (json["holderSystemId"] as? NSNumber)?.intValue
        takeoverAllowed = (json["takeoverAllowed"] as? NSNumber)?.boolValue
        systemManager = (json["systemManager"] as? NSNumber)?.boolValue
        requestAllowed = (json["requestAllowed"] as? NSNumber)?.boolValue ?? false
    }

    // SILENCE IS THE DESIGN HERE, not a head that forgot. QGC hides the indicator entirely until
    // firstControlStatusReceived (GCSControlIndicator.qml:15) because a vehicle that has never
    // sent CONTROL_STATUS reports sysidInControl 0 with both flags false, which is byte-identical
    // to another station holding it with takeover denied. The core says as much in its own
    // reason, and drawing THAT sentence would park "This vehicle has not said who is flying it."
    // in front of every single-station operator forever -- a permanent line about a question
    // nobody on a one-station field is asking.
    var worthShowing: Bool { available && known }

    static let thisStation = "This GCS"
    static let unreported = "Not reported"

    // QGC prints the bare system id for another station and "This GCS (id)" for ours
    // (GCSControlIndicator.qml:250). The id is kept in BOTH arms: it is the number the operator
    // reads off the other station's own title bar when they go to ask for control by radio.
    var holderText: String {
        guard let holderSystemId else { return OperatorControl.unreported }
        return inControl == true
            ? "\(OperatorControl.thisStation) (\(holderSystemId))"
            : "GCS \(holderSystemId)"
    }

    static let takeoverOn = "Allowed"
    static let takeoverOff = "Not allowed"

    var takeoverText: String {
        guard let takeoverAllowed else { return "" }
        return takeoverAllowed ? OperatorControl.takeoverOn : OperatorControl.takeoverOff
    }

    // inControl is null even with known true when this station's OWN MAVLink system id cannot be
    // read, because the answer is a comparison against it. That is NOT "another station is
    // flying" -- the core's reason collapses those two into one sentence -- so the head draws the
    // holder without the verdict and takes no colour from a comparison it could not make.
    var level: FlyTelemetry.Level {
        guard worthShowing else { return .unknown }
        switch inControl {
        case .some(true): return .good
        case .some(false): return .caution
        case .none: return .unknown
        }
    }
}
