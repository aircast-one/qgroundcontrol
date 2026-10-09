import Foundation

let SETUP = "view.setup"
let FIRMWARE = "view.firmware"

func parametersReady(_ view: JSON?) -> Bool {
    view?["connected"].bool == true && view?["parametersReady"].bool == true
}

struct SetupPage: Equatable, Hashable {
    let name: String
    let parameterSections: Bool
    var screen: String = ""
}

struct SetupGroup: Equatable {
    let title: String
    let pages: [SetupPage]
}

func setupGroups(_ view: JSON?) -> [SetupGroup] {
    (view?["groups"].objects ?? []).map { group in
        SetupGroup(
            title: group["title"].string,
            pages: group["pages"].objects.map {
                SetupPage(name: $0["name"].string, parameterSections: $0["parameterSections"].bool, screen: $0["screen"].string)
            }
        )
    }
}

struct SetupReadiness: Equatable {
    var ready: Bool?
    var setupComplete: Bool? = nil
    var headline: String
    var detail: String
    var connected: Bool
    var firmware: String
    var vehicleId: Int? = nil
}

let NOTHING_TO_CONFIGURE = "Nothing to Configure"
let NOTHING_TO_CONFIGURE_TEXT = "Aircast doesn't support setup for this vehicle type. If it is already configured, you can still fly."

func setupReadiness(_ view: JSON?) -> SetupReadiness? {
    guard let it = view, it.object != nil else { return nil }
    return SetupReadiness(
        ready: it["ready"].isNull ? nil : it["ready"].bool,
        setupComplete: it["setupComplete"].isNull ? nil : it["setupComplete"].bool,
        headline: it["headline"].string,
        detail: it["detail"].string,
        connected: it["connected"].bool,
        firmware: it["firmware"].string,
        vehicleId: it["vehicleId"].isNull ? nil : it["vehicleId"].int(0)
    )
}

func isPx4(_ readiness: SetupReadiness?) -> Bool { readiness?.firmware == "px4" }

func setupPage(_ view: JSON?, _ name: String) -> SetupPage? {
    setupGroups(view).flatMap(\.pages).first { $0.name == name }
}

func setupPagePath(_ name: String) -> String { "\(SETUP)(\(name))" }
