import Foundation

let MISSION_CONTEXT = "mission"
let ITEM_CONTEXT = "item"
let ALT_MODE_MIXED = 0
let FRAME_UNKNOWN = "Frame"

struct AltitudeModeOffer: Equatable {
    var raw: Int
    var title: String
    var help: String
    var enabled: Bool
    var current: Bool
    var reason: String
}

struct AltitudeModesView: Equatable {
    var context: String
    var current: Int
    var offers: [AltitudeModeOffer]
    var omitted: [AltitudeModeOffer]
}

func altitudeModesPath(_ context: String, _ current: Int) -> String { "view.altitudeModes(\(context),\(current))" }

private func offers(_ view: JSON?, _ key: String) -> [AltitudeModeOffer] {
    (view?[key].arrayOrNil ?? []).filter { $0.object != nil }.map {
        AltitudeModeOffer(
            raw: $0["raw"].int(-1),
            title: $0["title"].string,
            help: $0["help"].string,
            enabled: $0["enabled"].bool,
            current: $0["current"].bool,
            reason: $0["reason"].string
        )
    }
}

func altitudeModesView(_ view: JSON?) -> AltitudeModesView? {
    guard let view, view["class"].string == "AltitudeModes" else { return nil }
    return AltitudeModesView(
        context: view["context"].string,
        current: view["current"].int(-1),
        offers: offers(view, "modes"),
        omitted: offers(view, "omitted")
    )
}

func choosable(_ view: AltitudeModesView?) -> [AltitudeModeOffer] {
    (view?.offers ?? []).filter { $0.raw != ALT_MODE_MIXED }
}

func offersChoice(_ view: AltitudeModesView?) -> Bool { choosable(view).filter(\.enabled).count > 1 }

func refusalFor(_ view: AltitudeModesView?, _ raw: Int) -> String? {
    (view?.offers ?? []).first { $0.raw == raw }.flatMap { $0.enabled || $0.reason.isBlank ? nil : $0.reason }
}

func altitudeModePath(_ index: Int) -> String { "\(PLAN_ITEMS).\(index).altitudeMode" }
