import Foundation

struct FlightModeChoice: Identifiable, Equatable {
    let name: String
    let advanced: Bool
    let current: Bool

    var id: String { name }
    var summary: String { FlightModes.description(of: name) }
    var symbol: String { FlightModes.symbol(for: name) }
}

enum FlightModes {
    static let descriptions: [String: String] = [
        "Stabilize": "You fly it by hand, it only levels itself",
        "Stabilized": "You fly it by hand, it only levels itself",
        "Manual": "Sticks go straight to the motors, no help",
        "Acro": "Sticks set rotation rate, no self-levelling",
        "Altitude Hold": "Holds height, you steer",
        "Altitude": "Holds height, you steer",
        "Depth Hold": "Holds depth, you steer",
        "Position Hold": "Holds position and height, sticks nudge it",
        "Position": "Holds position and height, sticks nudge it",
        "Loiter": "Holds position and height, or circles it on a plane",
        "Hold": "Stops and holds where it is",
        "Brake": "Stops as fast as it can and holds",
        "Guided": "Flies to points you tap on the map",
        "Guided No GPS": "Accepts attitude commands without a position fix",
        "Auto": "Flies the uploaded mission",
        "Mission": "Flies the uploaded mission",
        "RTL": "Climbs, returns home and lands",
        "Return": "Climbs, returns home and lands",
        "Smart RTL": "Retraces its own path back home",
        "AutoRTL": "Follows the mission's landing sequence home",
        "Return to Groundstation": "Returns to the ground station",
        "Land": "Lands straight down where it is",
        "Precision Land": "Lands on the landing target",
        "Precision Landing": "Lands on the landing target",
        "Takeoff": "Climbs to takeoff height and holds",
        "Circle": "Circles the point below it",
        "Orbit": "Circles a point you choose",
        "Follow": "Follows the ground station or a beacon",
        "Follow Me": "Follows the ground station",
        "Drift": "Coordinated turns, like a plane",
        "Sport": "Rate control with height hold",
        "Flip": "Does one flip, then returns to the previous mode",
        "Throw": "Starts flying when thrown",
        "Autotune": "Tunes the controllers automatically, needs room",
        "Flow Hold": "Holds position with optical flow, no GPS",
        "ZigZag": "Sweeps between two points you record",
        "SystemID": "Injects test signals for system identification",
        "AutoRotate": "Helicopter autorotation after engine loss",
        "Avoid ADSB": "Dodges ADS-B traffic automatically",
        "Turtle": "Flips itself upright after a crash",
        "Cruise": "Holds heading and height, sticks trim",
        "FBW A": "Sticks set bank and pitch, wings stay level",
        "FBW B": "Sticks set height and heading",
        "Training": "Manual with bank and pitch limits",
        "Thermal": "Circles rising air automatically",
        "Autoland": "Lands on the runway automatically",
        "Loiter to QLand": "Circles, then lands as a quadcopter",
        "QuadPlane Stabilize": "Hovers by hand, it only levels itself",
        "QuadPlane Hover": "Hovers holding height, you steer",
        "QuadPlane Loiter": "Hovers holding position and height",
        "QuadPlane Land": "Lands as a quadcopter where it is",
        "QuadPlane RTL": "Returns home and lands as a quadcopter",
        "QuadPlane AutoTune": "Tunes the hover controllers automatically",
        "QuadPlane Acro": "Rate control while hovering",
        "Steering": "Sticks set speed and turn rate",
        "Learning": "Records waypoints as you drive",
        "Simple": "Sticks steer relative to where you stand",
        "Dock": "Drives onto the docking target",
        "Surface": "Rises to the surface",
        "Surftrak": "Holds a set distance above the seabed",
        "Motor Detection": "Works out motor order and direction",
        "Rattitude": "Levels near centre, rate control at full stick",
        "Offboard": "Controlled by a companion computer",
        "Ready": "Armed and waiting on the ground",
        "Initializing": "Booting, cannot fly yet",
    ]

    static func description(of mode: String) -> String {
        descriptions[mode] ?? ""
    }

    static let glyphs: [(keywords: [String], symbol: String)] = [
        (["rtl", "return"], "house"),
        (["land", "dock"], "arrow.down.to.line"),
        (["takeoff"], "arrow.up.to.line"),
        (["auto", "mission"], "list.bullet"),
        (["guided", "offboard"], "hand.tap"),
        (["loiter", "circle", "orbit", "hold", "brake", "position"], "circle.dashed"),
        (["follow"], "figure.walk"),
        (["acro", "sport", "flip", "rattitude", "turtle"], "arrow.triangle.2.circlepath"),
        (["altitude", "depth", "surface", "surftrak"], "arrow.up.arrow.down"),
        (["autotune", "systemid", "motor detection"], "slider.horizontal.3"),
        (["stabilize", "stabilized", "manual", "training", "steering"], "hand.raised"),
    ]

    static func symbol(for mode: String) -> String {
        let name = mode.lowercased()
        let match = glyphs.first { $0.keywords.contains { name.contains($0) } }
        return match?.symbol ?? "airplane"
    }

    static func choices(all: [String], advanced: [String], current: String) -> [FlightModeChoice] {
        let folded = Set(advanced)
        return all.map {
            FlightModeChoice(name: $0, advanced: folded.contains($0), current: $0 == current)
        }
    }

    static func needsConfirming(_ mode: String, flying: Bool,
                                rtlMode: String, landMode: String) -> Bool {
        guard flying, !mode.isEmpty else { return false }
        return mode == rtlMode || mode == landMode
    }

    static func everyday(_ choices: [FlightModeChoice]) -> [FlightModeChoice] {
        choices.filter { !$0.advanced || $0.current }
    }

    static func folded(_ choices: [FlightModeChoice]) -> [FlightModeChoice] {
        choices.filter { $0.advanced && !$0.current }
    }
}
