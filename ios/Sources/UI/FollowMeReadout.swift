import SwiftUI

struct FollowMeReadout: View {
    @QgcPath(FOLLOW_ME_VIEW) private var view
    @Environment(\.theme) private var theme
    @Environment(\.flyOsd) private var flyOsd

    var body: some View {
        let reading = followMeReading(view)
        if let label = followMeLabel(reading) {
            let sending = reading?.wouldSend == true
            Text(label)
                .font(.labelLarge)
                .lineLimit(2)
                .fixedSize(horizontal: false, vertical: true)
                .padding(.horizontal, 10)
                .padding(.vertical, 6)
                .background(osdBackdrop((sending ? theme.colors.secondaryContainer : theme.colors.surfaceVariant).opacity(0.92), flyOsd))
                .frame(maxWidth: 300, alignment: .leading)
        }
    }
}
