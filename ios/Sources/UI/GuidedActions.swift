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
    var reasonCode: String = ""
    var confirm = true
    var order = 0

    var ready: Bool { offer == "ready" }
    var shown: Bool { offer != "hidden" }
}

func guidedOffers(_ view: JSON?) -> [String: GuidedOffer] {
    guard let actions = view?["actions"].arrayOrNil else { return [:] }
    let offers = actions.enumerated().filter { $0.element.object != nil && !$0.element["id"].string.isBlank }.map { at, action in
        GuidedOffer(
            id: action["id"].string,
            title: action["title"].string,
            offer: action["offer"].string,
            reason: action["reason"].string,
            prompt: action["prompt"].string,
            destructive: action["destructive"].bool,
            carriesValue: action["carriesValue"].bool,
            option: action["option"].string,
            reasonCode: action["reasonCode"].string,
            confirm: action["confirm"].bool(true),
            order: at
        )
    }
    return Dictionary(offers.map { ($0.id, $0) }, uniquingKeysWith: { _, last in last })
}

struct PayloadProblem: Equatable {
    var code: String
    var text: String
    var blocking: Bool
    var fix: String
}

func payloadNextBay(_ view: JSON?) -> Int { view?["payload"]["next"].int(0) ?? 0 }

func payloadDropLabel(_ view: JSON?) -> String { view?["payload"]["label"].string ?? "" }

func payloadProblems(_ view: JSON?) -> [PayloadProblem] {
    (view?["payload"]["problems"].arrayOrNil ?? []).filter { $0.object != nil }.map {
        PayloadProblem(code: $0["code"].string, text: $0["text"].string, blocking: $0["blocking"].bool, fix: $0["fix"].string)
    }
}

func resumeFromSequence(_ view: JSON?) -> Int? {
    guard let view, !view["resumeFromSequence"].isNull else { return nil }
    let sequence = view["resumeFromSequence"].int(0)
    return sequence > 0 ? sequence : nil
}
