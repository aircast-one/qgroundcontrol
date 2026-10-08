import Foundation

let CLOSE_CHECKS = "view.closeChecks"

func closePrompts(_ view: JSON?) -> [String] {
    (view?["prompts"].array ?? []).filter { $0.object != nil }.map { $0["message"].string }.filter { !$0.isBlank }
}
