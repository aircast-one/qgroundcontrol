import Foundation

let MODE_SLOTS = "view.modeSlots"

struct ModeSlot: Equatable {
    var slot: Int
    var mode: String
    var live: Bool
}

struct ModeSlotsView: Equatable {
    var channel: Int
    var slots: [ModeSlot]
    var reason: String
    var activeSwitches: [String] = []
    var optionChannelsOn: [Int] = []
    var channelMonitor: Bool = false
    var activeParams: Set<String> = []
}

func modeSlotsView(_ view: JSON?) -> ModeSlotsView? {
    guard let view, view["available"].bool else { return nil }
    return ModeSlotsView(
        channel: view["channel"].int(0),
        slots: view["slots"].array.filter { $0.object != nil }.map {
            ModeSlot(slot: $0["slot"].int(0), mode: $0["mode"].string, live: $0["live"].bool)
        },
        reason: view["reason"].string,
        activeSwitches: view["activeSwitches"].strings,
        optionChannelsOn: view["channelOptions"].array.filter { $0.object != nil && $0["enabled"].bool }.map { $0["channel"].int(0) },
        channelMonitor: view["channelMonitor"].bool,
        activeParams: Set(view["activeParams"].strings)
    )
}

func liveSlotText(_ view: ModeSlotsView?) -> String? {
    guard let view, !view.slots.isEmpty else { return nil }
    guard let live = view.slots.first(where: \.live) else {
        return view.reason.ifBlank("The switch on channel \(view.channel) is not on any mode slot.")
    }
    return "The switch on channel \(view.channel) is on slot \(live.slot), \(live.mode)."
}

func liveSwitchesText(_ view: ModeSlotsView?) -> String? {
    guard let view else { return nil }
    if !view.activeSwitches.isEmpty { return "Switches on: \(view.activeSwitches.joined(separator: ", "))" }
    if !view.optionChannelsOn.isEmpty { return "Channel options on: \(view.optionChannelsOn.map { "channel \($0)" }.joined(separator: ", "))" }
    return nil
}
