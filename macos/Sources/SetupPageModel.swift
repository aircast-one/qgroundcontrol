import Foundation

enum SetupPage {
    static let sections: [(title: String, pages: [String])] = [
        ("Vehicle", ["Summary"]),
        ("Setup", ["Sensors", "Radio", "Frame", "Flight Modes", "Safety", "Power", "Motors",
                   "Tuning", "Camera"]),
        ("Advanced", ["Remote Support", "Parameters"]),
    ]

    static let all = sections.flatMap(\.pages)

    static func symbol(for page: String) -> String {
        switch page {
        case "Summary": return "airplane"
        case "Sensors": return "gauge"
        case "Flight Modes": return "slider.horizontal.3"
        case "Safety": return "shield.fill"
        case "Parameters": return "list.bullet"
        case "Radio": return "antenna.radiowaves.left.and.right"
        case "Frame": return "square.on.square"
        case "Power": return "bolt.fill"
        case "Motors": return "gearshape.2.fill"
        case "Camera": return "camera.fill"
        case "Remote Support": return "lifepreserver.fill"
        case "Tuning": return "dial.min"
        default: return "gearshape.fill"
        }
    }
}
