import Foundation

struct OrbitReading: Equatable {
    var available: Bool
    var orbiting: Bool?
    var radiusText: String
    var clockwise: Bool?
    var reason: String
}

func orbitReading(_ view: JSON?) -> OrbitReading? {
    guard let view, view["class"].string == "Orbit" else { return nil }
    return OrbitReading(
        available: view["available"].bool,
        orbiting: view["orbiting"].isNull ? nil : view["orbiting"].bool,
        radiusText: view["radiusText"].string,
        clockwise: view["clockwise"].isNull ? nil : view["clockwise"].bool,
        reason: view["reason"].string
    )
}

func orbitLabel(_ reading: OrbitReading?) -> String? {
    guard let reading, reading.orbiting == true else { return nil }
    let turn = reading.clockwise.map { $0 ? "clockwise" : "anticlockwise" } ?? ""
    return ["Orbiting", reading.radiusText, turn].filter { !$0.isBlank }.joined(separator: " ")
}
