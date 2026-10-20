import Foundation

let GUIDED_ACTIONS = "view.guidedActions"
let AUTODROP_STATUS = "view.autodropStatus"

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

struct PayloadBays: Equatable {
    var bays: Int
    var empty: [Int]
    var outputs: [Int]

    var multiBay: Bool { bays > 1 || !outputs.isEmpty }

    var numbers: [Int] { Array(1...max(bays, 1)) }

    func output(of bay: Int) -> String {
        outputs.indices.contains(bay - 1) ? "Bay \(bay) · output \(outputs[bay - 1])" : ""
    }

    func dropText(_ bay: Int) -> String {
        empty.contains(bay) ? "Bay \(bay) empty, hold to drop" : "Drop bay \(bay)"
    }
}

func payloadBays(_ view: JSON?) -> PayloadBays {
    let payload = view?["payload"]
    return PayloadBays(
        bays: max(payload?["bays"].int(1) ?? 1, 1),
        empty: (payload?["empty"].arrayOrNil ?? []).map { $0.int(0) },
        outputs: (payload?["outputs"].arrayOrNil ?? []).map { $0.int(0) }
    )
}

func payloadProblem(_ json: JSON) -> PayloadProblem {
    PayloadProblem(code: json["code"].string, text: json["text"].string, blocking: json["blocking"].bool, fix: json["fix"].string)
}

func payloadProblems(_ view: JSON?) -> [PayloadProblem] {
    (view?["payload"]["problems"].arrayOrNil ?? []).filter { $0.object != nil }.map(payloadProblem)
}

func resumeFromSequence(_ view: JSON?) -> Int? {
    guard let view, !view["resumeFromSequence"].isNull else { return nil }
    let sequence = view["resumeFromSequence"].int(0)
    return sequence > 0 ? sequence : nil
}
