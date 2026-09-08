import Foundation

struct ItemSpeed: Equatable {
    let available: Bool
    let specified: Bool
    let value: Double?
    let units: String

    static let unavailable = ItemSpeed(available: false, specified: false,
                                       value: nil, units: "m/s")

    init(available: Bool, specified: Bool, value: Double?, units: String) {
        self.available = available
        self.specified = specified
        self.value = value
        self.units = units
    }

    init(json: [String: Any]) {
        available = (json["available"] as? NSNumber)?.boolValue ?? false
        specified = (json["specifyFlightSpeed"] as? NSNumber)?.boolValue ?? false

        let fact = ((json["facts"] as? [[String: Any]]) ?? [])
            .first { ($0["name"] as? String) == ItemSpeed.factName }
        value = (fact?["value"] as? NSNumber)?.doubleValue
        units = (fact?["units"] as? String) ?? "m/s"
    }

    static let factName = "FlightSpeed"
    static let property = "flightSpeed"

    func shown(missionStart: Bool, vehicle: MissionVehicle) -> Bool {
        available && (!missionStart || vehicle.acceptsMissionStartSpeed)
    }

    var note: String {
        specified
            ? "This item flies at its own speed."
            : "This item flies at whatever speed the one before it set."
    }
}
