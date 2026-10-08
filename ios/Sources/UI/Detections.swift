import Foundation

let DETECTIONS = "view.detections"

struct DetectionBox: Equatable {
    var x: Double
    var y: Double
    var w: Double
    var h: Double
    var label: String
    var confidence: Double
    var target: Bool
}

struct Detections: Equatable {
    var available: Bool
    var stale: Bool
    var boxes: [DetectionBox]
    var error: String?
}

func detections(_ view: JSON?) -> Detections? {
    guard let view, view["class"].string == "Detections" else { return nil }
    return Detections(
        available: view["available"].bool,
        stale: view["stale"].bool,
        boxes: view["boxes"].array.filter { $0.object != nil }.map { box in
            DetectionBox(
                x: box["x"].double(0),
                y: box["y"].double(0),
                w: box["w"].double(0),
                h: box["h"].double(0),
                label: box["label"].string,
                confidence: box["confidence"].double(0),
                target: box["target"].bool
            )
        },
        error: view.has("error") && !view["error"].string.isBlank ? view["error"].string : nil
    )
}

func visibleBoxes(_ reading: Detections?) -> [DetectionBox] {
    reading.flatMap { $0.available && !$0.stale ? $0.boxes : nil } ?? []
}

let NO_FRAMES = "no frames from the detector"

func detectionTrouble(_ reading: Detections?) -> String? {
    guard let reading, reading.available else { return nil }
    return reading.error ?? (reading.stale ? NO_FRAMES : nil)
}

func boxCaption(_ box: DetectionBox) -> String {
    let percent = Int(box.confidence * 100)
    switch (box.label.isBlank, percent <= 0) {
    case (true, true): return ""
    case (true, false): return "\(percent)%"
    case (false, true): return box.label
    case (false, false): return "\(box.label) \(percent)%"
    }
}
