import os
import UIKit

let DEBUG_UI_ACTION = "debug-ui"
private let DEBUG_UI_STATE_FILE = "debug-ui-state.json"
private let debugUiLog = Logger(subsystem: "one.aircast.app", category: "DebugUi")

enum DebugCommand: Equatable {
    case ShowTab(Tab)
    case ShowFlyView(FlyView)
    case Orient(UIInterfaceOrientationMask)
    case EditLayout(Bool)
    case ResetLayout
    case Open(String)
}

struct DebugUiError: Error, Equatable {
    let message: String
}

private let ORIENTATIONS: [(String, UIInterfaceOrientationMask)] = [
    ("landscape", .landscapeRight),
    ("reverse-landscape", .landscapeLeft),
    ("portrait", .portrait),
    ("auto", .all),
]

private let SWITCHES = [("on", true), ("off", false)]

let DEBUG_COMMANDS = "state, tab, fly-view, orientation, layout-edit, layout-reset, open"

private func named<T>(_ value: String?, _ choices: [(String, T)], _ what: String) -> Result<T, DebugUiError> {
    choices.first { $0.0.caseInsensitiveCompare(value ?? "") == .orderedSame }.map { .success($0.1) }
        ?? .failure(DebugUiError(message: "\(what) must be one of \(choices.map(\.0).joined(separator: ", "))"))
}

func debugCommand(_ cmd: String?, _ value: String?) -> Result<DebugCommand, DebugUiError> {
    switch cmd {
    case "tab": named(value, Tab.allCases.map { ($0.rawValue, $0) }, "tab").map { .ShowTab($0) }
    case "fly-view": named(value, FlyView.allCases.map { ($0.rawValue, $0) }, "fly-view").map { .ShowFlyView($0) }
    case "orientation": named(value, ORIENTATIONS, "orientation").map { .Orient($0) }
    case "layout-edit": named(value, SWITCHES, "layout-edit").map { .EditLayout($0) }
    case "layout-reset": .success(.ResetLayout)
    case "open":
        value.flatMap { $0.isBlank ? nil : Result<DebugCommand, DebugUiError>.success(.Open($0)) }
            ?? .failure(DebugUiError(message: "open needs a sheet (\((REQUESTABLE_SHEETS + ["settings"]).joined(separator: ", "))) or a flight action id, e.g. takeoff, rtl, land"))
    default: .failure(DebugUiError(message: "cmd must be one of \(DEBUG_COMMANDS)"))
    }
}

@MainActor
enum DebugUi {
    static let commands = Notification.Name("one.aircast.app.\(DEBUG_UI_ACTION)")

    static var state = "{}" {
        didSet {
            guard CoreHost.isDebugBuild, state != oldValue else { return }
            let file = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0].appendingPathComponent(DEBUG_UI_STATE_FILE)
            try? Data(state.utf8).write(to: file, options: .atomic)
        }
    }

    static func orient(_ orientations: UIInterfaceOrientationMask) {
        UIApplication.shared.connectedScenes
            .compactMap { $0 as? UIWindowScene }
            .forEach { $0.requestGeometryUpdate(.iOS(interfaceOrientations: orientations)) }
    }
}

@MainActor
enum DebugUiReceiver {
    static func onReceive(_ url: URL) -> Bool {
        guard CoreHost.isDebugBuild, url.host == DEBUG_UI_ACTION else { return false }
        let query = URLComponents(url: url, resolvingAgainstBaseURL: false)?.queryItems ?? []
        let cmd = query.first { $0.name == "cmd" }?.value
        let result = cmd == "state" ? DebugUi.state : {
            switch debugCommand(cmd, query.first { $0.name == "value" }?.value) {
            case .success(let command):
                NotificationCenter.default.post(name: DebugUi.commands, object: command)
                return "ok"
            case .failure(let refusal):
                return "error: \(refusal.message)"
            }
        }()
        debugUiLog.info("\(result, privacy: .public)")
        return true
    }
}
