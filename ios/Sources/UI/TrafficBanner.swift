import SwiftUI

private let TRAFFIC_BANNER_MAX_WIDTH: CGFloat = 560
private let TRAFFIC_BANNER_CORNER: CGFloat = 12

struct TrafficBanner: View {
    @QgcPath(TRAFFIC_VIEW) private var view
    @State private var listed = false
    @Environment(\.theme) private var theme

    var body: some View {
        let reading = trafficReading(view)
        ZStack {
            OpenOnRequest(name: TRAFFIC_SHEET) { listed = true }
            if let reading, listed {
                TrafficSheet(reading: reading) { listed = false }
            }
            if let reading, let alert = trafficAlert(reading) {
                banner(reading, alert)
            }
        }
    }

    private func banner(_ reading: TrafficReading, _ alert: TrafficAlert) -> some View {
        let urgent = alert.level >= .Warning
        return Button { listed = true } label: {
            HStack(spacing: Space.s3) {
                Image(urgent ? .flight : .warning)
                    .resizable()
                    .scaledToFit()
                    .foregroundStyle(urgent ? theme.colors.error : theme.aircast.warning)
                    .frame(width: 24, height: 24)
                VStack(alignment: .leading, spacing: 0) {
                    Text(alert.title).font(.titleSmall).lineLimit(1).truncationMode(.tail)
                    if !alert.detail.isBlank {
                        Text(alert.detail).font(.bodyMedium).lineLimit(1).truncationMode(.tail)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .padding(.horizontal, Space.s3)
            .padding(.vertical, Space.s2)
            .frame(minHeight: 48)
            .foregroundStyle(urgent ? theme.colors.onErrorContainer : theme.colors.onSurface)
            .background(urgent ? theme.colors.errorContainer : theme.aircast.warningContainer, in: RoundedRectangle(cornerRadius: TRAFFIC_BANNER_CORNER))
        }
        .buttonStyle(.plain)
        .allowsHitTesting(!reading.contacts.isEmpty)
        .frame(maxWidth: TRAFFIC_BANNER_MAX_WIDTH)
    }
}

private struct TrafficSheet: View {
    let reading: TrafficReading
    let onDismiss: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        AircastSheet(onDismissRequest: onDismiss) {
            Text(trafficSummary(reading))
                .font(.titleSmall)
                .padding(.horizontal, Space.s5)
            Text(trafficCaption(reading))
                .font(.bodySmall)
                .foregroundStyle(theme.colors.onSurfaceVariant)
                .padding(.horizontal, Space.s5)
                .padding(.vertical, Space.s1)
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 0) {
                    ForEach(reading.contacts) { contact in
                        let colour = trafficContactUrgent(contact) ? theme.colors.error : theme.colors.onSurface
                        HStack(alignment: .firstTextBaseline, spacing: Space.s3) {
                            Text(contact.name).font(.labelLarge).foregroundStyle(colour)
                            Text(trafficContactText(contact, reading.units))
                                .font(.bodyMedium)
                                .foregroundStyle(colour)
                                .frame(maxWidth: .infinity, alignment: .leading)
                        }
                        .padding(.horizontal, Space.s5)
                        .padding(.vertical, 6)
                    }
                }
                .padding(.bottom, Space.s6)
            }
        }
    }
}
