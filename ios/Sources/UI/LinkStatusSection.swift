import SwiftUI

let LINK_STATUS_VIEW = "view.linkStatus"

func linkStatusRows(_ view: JSON?) -> [(String, String)] {
    (view?["rows"].arrayOrNil ?? []).filter { $0.object != nil }.map { ($0["label"].string, $0["value"].string) }
}

struct LinkStatusSection: View {
    @Environment(\.theme) private var theme
    @QgcPath(LINK_STATUS_VIEW) private var json

    var body: some View {
        let rows = linkStatusRows(json)
        SectionHeader(text: "Link Status (Current Vehicle)")
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
