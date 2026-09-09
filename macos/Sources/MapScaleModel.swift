import Foundation

struct MapScaleBar: Equatable {
    let text: String
    let fraction: Double

    static let none = MapScaleBar(text: "", fraction: 0)

    init(text: String, fraction: Double) {
        self.text = text
        self.fraction = fraction
    }

    init?(_ json: [String: Any]) {
        guard (json["available"] as? NSNumber)?.boolValue == true,
              let fraction = (json["fraction"] as? NSNumber)?.doubleValue else { return nil }
        text = (json["text"] as? String) ?? ""
        self.fraction = fraction
    }

    static func across(_ metresAcross: Double) -> Int? {
        guard metresAcross > 0, metresAcross.isFinite,
              metresAcross < Double(Int.max) else { return nil }
        return Int(metresAcross)
    }
}
