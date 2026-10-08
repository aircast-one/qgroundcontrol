import Foundation

enum SetupPage {
    static let unknownSymbol = "gearshape.fill"

    static let glyphs: [String: String] = [
        "Summary": "airplane",
        "Sensors": "gauge",
        "Flight Modes": "slider.horizontal.3",
        "Safety": "shield.fill",
        "Parameters": "list.bullet",
        "Radio": "antenna.radiowaves.left.and.right",
        "Frame": "square.on.square",
        "Power": "bolt.fill",
        "Motors": "gearshape.2.fill",
        "Lights": "lightbulb.fill",
        "Remote Support": "lifepreserver.fill",
        "Tuning": "dial.min",
        "Flight Behavior": "wind",
        "Actuators": "slider.vertical.3",
        "Heli": "fanblades",
        "Follow Me": "figure.walk",
        "WiFi Bridge": "wifi",
        "Syslink": "link",
        "Flight Safety": "checkmark.shield.fill",
        "Failsafes": "exclamationmark.shield.fill",
        "Airspeed": "speedometer",
        "ESC": "cpu",
        "Servo Outputs": "slider.horizontal.below.rectangle",
        "Tuning - Advanced": "dial.max",
        "Gimbal": "camera.fill",
        "Joystick": "gamecontroller.fill",
        "Logging": "doc.text.fill",
        "Scripting": "chevron.left.forwardslash.chevron.right",
    ]

    static func symbol(for page: String) -> String {
        glyphs[page] ?? unknownSymbol
    }

    static let bespoke: Set<String> = [
        "Summary", "Parameters", "Safety", "Power", "Frame", "Radio", "Tuning",
        "Lights", "Motors", "Remote Support", "Flight Modes", "Sensors",
    ]

    static func draws(_ page: SetupPageInfo) -> Bool {
        bespoke.contains(page.name) || page.parameterSections
    }
}
