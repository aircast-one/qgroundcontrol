import SwiftUI

let LINK_STATUS_VIEW = "view.linkStatus"

func linkStatusRows(_ view: JSON?) -> [(String, String)] {
    (view?["rows"].objects ?? []).map { ($0["label"].string, $0["value"].string) }
}

private let LINK_STATUS_TITLE = "Link Status (Current Vehicle)"

// Inside a NativeForm: a section of its own, a value row per reading.
struct LinkStatusSection: View {
    var heading: String = ""
    @Environment(\.theme) private var theme
    @Environment(\.LocalNativeList) private var nativeList
    @QgcPath(LINK_STATUS_VIEW) private var json

    var body: some View {
        if nativeList {
            native
        } else {
            handDrawn
        }
    }

    @ViewBuilder private var native: some View {
        let rows = linkStatusRows(json)
        Section {
            if rows.isEmpty {
                Text("Not connected").foregroundStyle(Color.secondary)
            }
            ForEach(Array(rows.enumerated()), id: \.offset) { _, row in
                LabeledContent(row.0, value: row.1)
            }
        } header: {
            SectionHeader(text: heading.ifBlank(LINK_STATUS_TITLE))
        }
    }

    @ViewBuilder private var handDrawn: some View {
        let rows = linkStatusRows(json)
        SectionHeader(text: LINK_STATUS_TITLE)
        if rows.isEmpty {
            Text("Not connected")
                .font(.bodyMedium)
                .foregroundStyle(theme.colors.onSurfaceVariant)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, Space.s4)
        }
        ForEach(Array(rows.enumerated()), id: \.offset) { _, row in
            HStack {
                Text(row.0).font(.bodyLarge).frame(maxWidth: .infinity, alignment: .leading)
                Text(row.1).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
            }
            .padding(.horizontal, Space.s4)
            .padding(.vertical, 6)
        }
    }
}
