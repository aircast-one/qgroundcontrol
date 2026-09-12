import Foundation

struct DetectionBox: Equatable {
    let label: String
    let confidence: Double?
    let x: Double
    let y: Double
    let width: Double
    let height: Double

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let label = json["label"] as? String, !label.isEmpty,
              let x = (json["x"] as? NSNumber)?.doubleValue,
              let y = (json["y"] as? NSNumber)?.doubleValue,
              let width = (json["w"] as? NSNumber)?.doubleValue,
              let height = (json["h"] as? NSNumber)?.doubleValue,
              x.isFinite, y.isFinite, width.isFinite, height.isFinite,
              width > 0, height > 0 else { return nil }
        self.label = label
        self.x = x
        self.y = y
        self.width = width
        self.height = height
        let reported = (json["confidence"] as? NSNumber)?.doubleValue
        confidence = (reported?.isFinite ?? false) ? reported : nil
    }

    var caption: String {
        guard let confidence else { return label }
        return "\(label) \(Int((confidence * 100).rounded()))%"
    }
}

struct Detections: Equatable {
    let available: Bool
    let stale: Bool
    let boxes: [DetectionBox]
    let error: String

    static let none = Detections()

    private init() {
        available = false
        stale = true
        boxes = []
        error = ""
    }

    init(_ json: [String: Any]) {
        available = (json["available"] as? NSNumber)?.boolValue ?? false
        stale = (json["stale"] as? NSNumber)?.boolValue ?? true
        boxes = ((json["boxes"] as? [Any]) ?? []).compactMap(DetectionBox.init)
        error = (json["error"] as? String) ?? ""
    }

    var draws: Bool { available && !stale && !boxes.isEmpty }
}
