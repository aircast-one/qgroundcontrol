import SwiftUI

let GLOBAL_FRAME_MIXED = 0
let GLOBAL_FRAME_RELATIVE = 1

func itemReferenceShown(_ globalFrame: Int?) -> Bool { globalFrame != GLOBAL_FRAME_RELATIVE }

func itemReferenceSelectable(_ globalFrame: Int?) -> Bool { globalFrame == nil || globalFrame == GLOBAL_FRAME_MIXED }

struct AltitudeModePicker: View {
    let item: MissionItem
    let onPick: (Int) -> Void
    var globalFrameMixed: Bool = true
    @MapPath private var json: JSON?

    init(item: MissionItem, onPick: @escaping (Int) -> Void, globalFrameMixed: Bool = true) {
        self.item = item
        self.onPick = onPick
        self.globalFrameMixed = globalFrameMixed
        _json = MapPath(altitudeModesPath(ITEM_CONTEXT, item.altitudeMode))
    }

    var body: some View {
        let view = altitudeModesView(json)
        let picks = choosable(view)
        let live = globalFrameMixed && offersChoice(view)
        let notes = picks.filter { !$0.current }.compactMap { offer in refusalFor(view, offer.raw).map { "\(offer.title): \($0)" } }
        VStack(alignment: .leading, spacing: 2) {
            if picks.isEmpty {
                Text(item.altitudeFrameText.ifBlank(FRAME_UNKNOWN)).font(.labelMedium)
            }
            PlanFlowRow(spacing: 8, lineSpacing: 0) {
                ForEach(picks, id: \.raw) { offer in
                    PlanChip(label: sentenceCase(offer.title), selected: offer.current, enabled: live && offer.enabled || offer.current) {
                        if live && !offer.current { onPick(offer.raw) }
                    }
                }
            }
            if live {
                ForEach(notes, id: \.self) { Text($0).font(.labelSmall) }
            }
        }
    }
}
