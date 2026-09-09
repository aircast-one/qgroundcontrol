import Foundation

enum SetupPage {
    static let unknownSymbol = "gearshape.fill"

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
        case "Lights": return "lightbulb.fill"
        case "Remote Support": return "lifepreserver.fill"
        case "Tuning": return "dial.min"
        case "Flight Behavior": return "wind"
        default: return unknownSymbol
        }
    }
}
