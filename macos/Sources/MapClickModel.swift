import Foundation

struct MapClickState: Equatable {
    var connected = false
    var flying = false
    var missionActive = false
    var roiSupported = false
    // Read and reported but no longer a gate: shown(.orbit) refuses before reaching it. Kept
    // because it is a fact about the vehicle and the orbit needs it back the day this head can
    // ask an operator for a radius and a height.
    var orbitSupported = false

    // What the core says a goto should carry: the operator's setting when the vehicle flies
    // forward, and zero otherwise. A fixed wing cannot hover, so a goto has to say how wide to
    // circle; sending a multirotor a radius would be sending a number nobody chose. Read, never
    // derived - a radius invented here is the same defect one layer down.
    var gotoLoiterRadius: Double = 0
    var homeUsable = false
    var gpsSensorPresent = true
    var inGotoMode = false
    var confirmGotoInGuided = true
    var roiActive = false

    static let gpsSensorBit = 32

    static func homeUsable(_ point: GeoPoint?, altitude: Double?) -> Bool {
        guard let point, let altitude, altitude.isFinite else { return false }
        return point.latitude != 0 || point.longitude != 0
    }
}

enum MapClickAction: String, CaseIterable, Identifiable {
    case goTo
    case orbit
    case roi
    case cancelRoi
    case setHome
    case setHeading
    case setEstimatorOrigin

    var id: String { rawValue }

    var title: String {
        switch self {
        case .goTo: return "Fly to here"
        case .orbit: return "Orbit here"
        case .roi: return "Look at here"
        case .cancelRoi: return "Stop looking"
        case .setHome: return "Set home here"
        case .setHeading: return "Face this way"
        case .setEstimatorOrigin: return "Set estimator origin"
        }
    }

    var prompt: String {
        switch self {
        case .goTo: return "Fly to the point you picked, at the height it is at now?"
        case .orbit: return "Circle the point you picked?"
        case .roi: return "Point the camera at this spot and keep it there?"
        case .cancelRoi: return "Stop pointing the camera at that spot?"
        case .setHome: return "Move the vehicle's home position to this point?"
        case .setHeading: return "Turn the vehicle to face this point?"
        case .setEstimatorOrigin: return "Tell the vehicle it is starting from this point?"
        }
    }

    var invokable: String {
        switch self {
        case .goTo: return "guidedModeGotoLocation"
        case .orbit: return "guidedModeOrbit"
        case .roi: return "guidedModeROI"
        case .cancelRoi: return "stopGuidedModeROI"
        case .setHome: return "doSetHome"
        case .setHeading: return "guidedModeChangeHeading"
        case .setEstimatorOrigin: return "setEstimatorOrigin"
        }
    }

    func shown(in state: MapClickState) -> Bool {
        guard state.connected else { return false }
        switch self {
        case .goTo: return state.flying
        case .orbit:
            // Not offered, and not because the vehicle cannot do it. guidedModeOrbit takes a
            // radius and an AMSL altitude, and this head asks the operator for neither - it sent
            // (centre, 0, 0), where the radius carries the turn direction in its sign and the
            // altitude is metres above SEA LEVEL. QML sends orbitMapCircle.radius() signed by
            // clockwiseRotation, and homePosition.altitude plus the height the operator picked.
            //
            // A guided command to a flying aircraft with two invented parameters is the shape
            // Clear Mission is withheld for: it fails by doing something, not by refusing. It
            // comes back when the core serves the radius and the altitude, the way it now serves
            // gotoLoiterRadius, and when there is somewhere for the operator to choose them.
            return false
        case .roi: return state.flying && state.roiSupported
        case .cancelRoi: return state.roiSupported && state.roiActive
        case .setHome: return true
        case .setHeading: return state.flying
        case .setEstimatorOrigin: return !state.gpsSensorPresent
        }
    }

    func needsConfirmation(in state: MapClickState) -> Bool {
        guard self == .goTo else { return true }
        return !(state.inGotoMode && !state.confirmGotoInGuided)
    }

    static func offered(in state: MapClickState) -> [MapClickAction] {
        allCases.filter { $0.shown(in: state) }
    }

    static func refusal(in state: MapClickState) -> String {
        state.connected
            ? "Nothing can be commanded from the map while the vehicle is on the ground."
            : "No vehicle is connected."
    }
}

enum MapMenuPlacement {
    static let gap = 18.0
    static let margin = 12.0

    static func place(click: Double, extent: Double, container: Double) -> Double {
        let low = margin
        let high = container - margin - extent
        guard high > low else { return low }
        let after = click + gap
        guard after > high else { return max(low, after) }
        return max(low, min(click - gap - extent, high))
    }
}
