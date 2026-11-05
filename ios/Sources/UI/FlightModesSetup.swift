import SwiftUI

struct FlightModesSetup: View {
    @QgcPath(MODE_SLOTS) private var view

    var body: some View {
        let slots = modeSlotsView(view)
        let live = [liveSlotText(slots), liveSwitchesText(slots)].compactMap { $0 }.joined(separator: "\n")
        ParameterForm(
            page: FLIGHT_MODES_PAGE,
            highlighted: slots?.activeParams ?? [],
            header: live.isBlank ? nil : AnyView(Text(live).font(.subheadline)),
            footer: slots?.channelMonitor == true ? AnyView(ChannelMonitor()) : nil
        )
    }
}

// The live channels under the flight-mode slots, a row per channel in the footer section of the page's form.
private struct ChannelMonitor: View {
    @QgcPath(RADIO_VIEW) private var json

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("Channel monitor").font(.headline).padding(.bottom, Space.s1)
            ForEach(radioView(json)?.channels ?? [], id: \.label) { channel in
                HStack(spacing: 0) {
                    Text(channel.label).font(.footnote).frame(width: 28, alignment: .leading)
                    PwmBar(fraction: channel.fraction)
                }
                .padding(.vertical, Space.s1)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .task { _ = await offMain { Qgc.invoke(radioCalAction("start")) } }
    }
}
