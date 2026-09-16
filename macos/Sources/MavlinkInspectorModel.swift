import Foundation

struct MavlinkMessage: Identifiable, Equatable {
    let index: Int
    let path: String
    let messageId: Int
    let name: String
    let componentId: Int
    let title: String
    let count: Int
    let rateText: String
    let targetRateHz: Int
    let targetRateTitle: String
    let selected: Bool

    // Two components can send the same message id — two cameras both reporting
    // CAMERA_CAPTURE_STATUS is the case that crashed the Android head — so rows are
    // identified by their path and only the title is disambiguated for reading.
    var id: String { path }

    var countText: String { "\(count)" }

    // The core appends "(comp N)" to the title ONLY when a name repeats -- the suffix exists to
    // disambiguate and nothing else in the row does it. The list card is 260pt wide and drew the
    // whole title on one line, so on a long name that suffix was the FIRST thing truncation ate:
    // two CAMERA_CAPTURE_STATUS rows at #262, identical on screen, differing only in the part cut
    // off. The second line already carries the message id and has room to spare.
    //
    // Keyed on the core HAVING disambiguated -- title differing from name -- rather than on the
    // head recounting the duplicates itself. Which names repeat is the judgement the producer
    // already made over the whole list, and a row cannot see the list.
    var disambiguated: Bool { title != name }

    var listTitle: String { name }

    var listDetail: String {
        disambiguated ? "#\(messageId)  \u{00B7}  comp \(componentId)" : "#\(messageId)"
    }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let path = json["path"] as? String, !path.isEmpty,
              let name = json["name"] as? String, !name.isEmpty else { return nil }
        self.path = path
        self.name = name
        index = (json["index"] as? NSNumber)?.intValue ?? 0
        messageId = (json["id"] as? NSNumber)?.intValue ?? 0
        componentId = (json["compId"] as? NSNumber)?.intValue ?? 0
        title = (json["title"] as? String) ?? name
        count = (json["count"] as? NSNumber)?.intValue ?? 0
        rateText = (json["rateText"] as? String) ?? ""
        targetRateHz = (json["targetRateHz"] as? NSNumber)?.intValue ?? 0
        targetRateTitle = (json["targetRateTitle"] as? String) ?? ""
        selected = (json["selected"] as? NSNumber)?.boolValue ?? false
    }

    static func list(_ json: Any?) -> [MavlinkMessage] {
        ((json as? [Any]) ?? []).compactMap(MavlinkMessage.init)
    }
}

struct MessageRateChoice: Identifiable, Equatable {
    let rate: Int
    let title: String

    var id: Int { rate }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let rate = (json["rate"] as? NSNumber)?.intValue else { return nil }
        self.rate = rate
        title = (json["title"] as? String) ?? ""
    }

    static func list(_ json: Any?) -> [MessageRateChoice] {
        ((json as? [Any]) ?? []).compactMap(MessageRateChoice.init)
    }

    // core-rs inspector.rs RATE_DISABLED and RATE_DEFAULT, which it titles Off and Default.
    static let offRate = -1
    static let defaultRate = 0

    static func offered(_ rate: Int, in choices: [MessageRateChoice]) -> Bool {
        choices.contains { $0.rate == rate }
    }

    // The same fallback as the core's shown_rate. It has to land on a rate the picker lists, or
    // the selection binds to a tag that is not there and the control draws blank.
    static func shown(_ rate: Int, in choices: [MessageRateChoice]) -> Int {
        offered(rate, in: choices) ? rate : defaultRate
    }
}

struct MavlinkField: Identifiable, Equatable {
    let name: String
    let type: String
    let value: String

    var id: String { name }

    init?(json: Any?) {
        guard let object = json as? [String: Any],
              let name = object["name"] as? String, !name.isEmpty else { return nil }
        self.name = name
        type = (object["type"] as? String) ?? ""
        value = (object["value"] as? String) ?? ""
    }

    static func from(_ elements: [Any]) -> [MavlinkField] {
        elements.compactMap(MavlinkField.init(json:))
    }
}

enum InspectorList {
    // Two different silences, and only one of them had a sentence. The core now spells both
    // (inspector.rs:41): "Connect a vehicle to inspect its MAVLink traffic." when no system is
    // there, "Waiting for this vehicle's first message..." when one is and nothing has arrived --
    // MAVLinkInspectorController.cc:66 connects messageReceived in its CONSTRUCTOR, so the list
    // begins empty and fills on the next frame. Which silence the vehicle is in is a vehicle fact
    // and theirs; WHEN a sentence is drawn at all is display policy and stays here.
    //
    // Gated on their text and not on the row count, so there is one source of truth: they serve ""
    // -- not null -- once there are rows, and that empty string IS "draw the list". A second gate
    // on count would be a branch whose arms agree until they disagree, and then the head would be
    // picking which of the two to believe.
    //
    // nil is the third state and is NOT "there are rows": it means the producer served no sentence,
    // which at HEAD happens only when the view itself failed to read. Falling through to the list
    // there would draw the bare empty card this whole enum exists to prevent, so it falls back to
    // the prompt the vibration and console panels already use for their unreadable states.
    static func emptyText(served: String?, count: Int) -> String? {
        guard let served else {
            return count == 0 ? VehicleSetupText.connectPrompt(for: "MAVLink traffic") : nil
        }
        return served.isEmpty ? nil : served
    }
}
