import Foundation

let GUIDED_ACTIONS = "view.guidedActions"

struct GuidedOffer: Equatable {
    var id: String
    var title: String
    var offer: String
    var reason: String
    var prompt: String
    var destructive: Bool
    var carriesValue: Bool
    var option: String = ""

    var ready: Bool { offer == "ready" }
    var shown: Bool { offer != "hidden" }
}

func guidedOffers(_ view: JSON?) -> [String: GuidedOffer] {
    guard let actions = view?["actions"].arrayOrNil else { return [:] }
    let offers = actions.filter { $0.object != nil && !$0["id"].string.isBlank }.map { action in
        GuidedOffer(
            id: action["id"].string,
            title: action["title"].string,
            offer: action["offer"].string,
            reason: action["reason"].string,
            prompt: action["prompt"].string,
            destructive: action["destructive"].bool,
            carriesValue: action["carriesValue"].bool,
            option: action["option"].string
        )
    }
    return Dictionary(offers.map { ($0.id, $0) }, uniquingKeysWith: { _, last in last })
}

func resumeFromSequence(_ view: JSON?) -> Int? {
    guard let view, !view["resumeFromSequence"].isNull else { return nil }
    let sequence = view["resumeFromSequence"].int(0)
    return sequence > 0 ? sequence : nil
}
