import SwiftUI

struct FlightModesSetup: View {
    @QgcPath(MODE_SLOTS) private var view
    @Environment(\.theme) private var theme

    var body: some View {
        let slots = modeSlotsView(view)
        let live = [liveSlotText(slots), liveSwitchesText(slots)].compactMap { $0 }.joined(separator: "\n")
        VStack(spacing: 0) {
            if !live.isBlank {
                Text(live)
                    .font(.bodyMedium)
                    .padding(.horizontal, Space.s4)
                    .padding(.vertical, 10)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(theme.colors.surfaceVariant)
            }
            ParameterForm(page: FLIGHT_MODES_PAGE, highlighted: slots?.activeParams ?? [])
            if slots?.channelMonitor == true { ChannelMonitor() }
        }
    }
}

private struct ChannelMonitor: View {
    @QgcPath(RADIO_VIEW) private var json

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("Channel monitor").font(.titleSmall)
            ForEach(radioView(json)?.channels ?? [], id: \.label) { channel in
                HStack(spacing: 0) {
                    Text(channel.label).font(.bodySmall).frame(width: 28, alignment: .leading)
                    PwmBar(fraction: channel.fraction)
                }
                .padding(.vertical, Space.s1)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, Space.s5)
        .padding(.vertical, Space.s2)
        .task { _ = await offMain { Qgc.invoke(radioCalAction("start")) } }
    }
}
