import Foundation

struct EscCalibrationReading: Equatable {
    static let dialog = "escCalibration"
    static let starting = "Starting ESC calibration..."

    let highlight: String
    let text: String
    let running: Bool

    init?(_ json: [String: Any]) {
        guard (json["open"] as? NSNumber)?.boolValue == true else { return nil }
        highlight = (json["highlight"] as? String) ?? ""
        text = (json["text"] as? String) ?? ""
        running = (json["running"] as? NSNumber)?.boolValue ?? false
    }
}
