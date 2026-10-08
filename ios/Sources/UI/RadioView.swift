import Foundation

let RADIO_VIEW = "view.radio"

struct RadioStick: Equatable {
    var title: String
    var valueText: String
    var fraction: Float
    var mapped: Bool
    var reversed: Bool
    var channel: Int? = nil
}

struct RadioChannel: Equatable {
    var label: String
    var valueText: String
    var fraction: Float
    var live: Bool
}

struct RadioCalibration: Equatable {
    var running: Bool
    var statusText: String
    var nextText: String
    var nextEnabled: Bool
    var cancelEnabled: Bool
    var skipEnabled: Bool
    var throttleReversed: Bool = false
    var stickPositions: [Int] = [0, 0, 0, 0]
}

struct RadioView {
    var connected: Bool
    var channelCount: Int
    var summary: String
    var calibration: RadioCalibration
    var transmitterMode: Int
    var centeredThrottle: Bool = false
    var joystickMode: Bool = false
    var sticks: [RadioStick]
    var channels: [RadioChannel]
    var startPrompt: (first: String, second: String)? = nil
    var notReady: (first: String, second: String)? = nil
}

private func each<T>(_ view: JSON, _ key: String, _ make: (JSON) -> T) -> [T] {
    (view[key].arrayOrNil ?? []).filter { $0.object != nil }.map(make)
}

private func titled(_ json: JSON) -> (first: String, second: String)? {
    json.object == nil ? nil : (json["title"].string, json["message"].string)
}

func radioView(_ view: JSON?) -> RadioView? {
    guard let view, view["class"].string == "Radio" else { return nil }
    let positions = view["stickPositions"].arrayOrNil?.map { $0.int(0) }
    return RadioView(
        connected: view["connected"].bool,
        channelCount: view["channelCount"].int(0),
        summary: view["summary"].string,
        calibration: RadioCalibration(
            running: view["calibrating"].bool,
            statusText: view["statusText"].string,
            nextText: view["nextText"].string,
            nextEnabled: view["nextEnabled"].bool,
            cancelEnabled: view["cancelEnabled"].bool,
            skipEnabled: view["skipEnabled"].bool,
            throttleReversed: view["throttleReversed"].bool,
            stickPositions: positions.flatMap { $0.count == 4 ? $0 : nil } ?? [0, 0, 0, 0]
        ),
        transmitterMode: view["transmitterMode"].int(2),
        centeredThrottle: view["centeredThrottle"].bool,
        joystickMode: view["joystickMode"].bool,
        sticks: each(view, "sticks") {
            RadioStick(
                title: $0["title"].string,
                valueText: $0["valueText"].string,
                fraction: Float($0["fraction"].double(0)),
                mapped: $0["mapped"].bool,
                reversed: $0["reversed"].bool,
                channel: $0["channel"].int(0) > 0 ? $0["channel"].int(0) : nil
            )
        },
        channels: each(view, "channels") {
            RadioChannel(
                label: $0["label"].string,
                valueText: $0["valueText"].string,
                fraction: Float($0["fraction"].double(0)),
                live: $0["live"].bool
            )
        },
        startPrompt: titled(view["startPrompt"]),
        notReady: titled(view["notReady"])
    )
}

let RADIO_CAL = "radioCal"

func radioCalAction(_ action: String) -> String { "\(RADIO_CAL).\(action)" }

func calibrationStep(_ statusText: String) -> String {
    let lines = statusText.trimmed.split(omittingEmptySubsequences: false, whereSeparator: \.isNewline).map(String.init)
    let kept = lines.reversed().drop { $0.isBlank || $0.trimmed.hasPrefix("Click ") }.reversed()
    return kept.joined(separator: "\n").trimmed
}

struct RadioPrompt: Equatable {
    var title: String
    var body: String
    var action: String
    var choices: [String]
}

let RADIO_PROMPTS = [
    RadioPrompt(
        title: "Spektrum bind",
        body: "Click Ok to place your Spektrum receiver in the bind mode.\n\nSelect the specific receiver type below:",
        action: "spektrumBindMode",
        choices: ["DSM2 Mode", "DSMX (7 channels or less)", "DSMX (8 channels or more)"]
    ),
    RadioPrompt(
        title: "CRSF bind",
        body: "Click Ok to place your CRSF receiver in the bind mode.",
        action: "crsfBindMode",
        choices: []
    ),
    RadioPrompt(
        title: "Copy trims",
        body: "Center your sticks and move throttle all the way down, then press Ok to copy trims. After pressing Ok, reset the trims on your radio back to zero.",
        action: "copyTrims",
        choices: []
    ),
]

let THROTTLE_REVERSED_TITLE = "Throttle channel reversed"
let THROTTLE_REVERSED_TEXT = "Calibration failed. The throttle channel on your transmitter is reversed. You must correct this on your transmitter in order to complete calibration."
