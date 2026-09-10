import Foundation

enum SetupPage {
    static let unknownSymbol = "gearshape.fill"

    // A dictionary rather than a switch so the head's page set is data a test can read. The
    // contract asserts it against view.setup.groups[].pages[].name both ways: a page the core
    // adds must gain a glyph here, and a name here that the core never lists is a page this head
    // still believes in. A switch could answer neither question.
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
        "Camera": "camera.fill",
        "Lights": "lightbulb.fill",
        "Remote Support": "lifepreserver.fill",
        "Tuning": "dial.min",
        "Flight Behavior": "wind",
    ]

    static func symbol(for page: String) -> String {
        glyphs[page] ?? unknownSymbol
    }

    // Every name VehicleSetupWindow's content switch has a case for. swift-checks.sh compares
    // this against that switch, because a name added to one and not the other is either a page
    // offered that opens on nothing or a page built that is never listed.
    static let bespoke: Set<String> = [
        "Summary", "Parameters", "Safety", "Power", "Frame", "Radio", "Tuning", "Camera",
        "Lights", "Motors", "Remote Support", "Flight Modes", "Sensors",
    ]

    static func draws(_ page: SetupPageInfo) -> Bool {
        bespoke.contains(page.name) || page.parameterSections
    }
}
