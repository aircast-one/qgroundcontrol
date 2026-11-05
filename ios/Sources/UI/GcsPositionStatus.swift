import SwiftUI

let REMOTE_ID_GROUP = "remoteIDSettings"
let GCS_LOCATION_BLOCK = "Ground Station Location"
private let GCS_POSITION = "positionManager.gcsPosition"
private let GCS_ACCURACY = "positionManager.gcsPositionHorizontalAccuracy"
private let GCS_POLL_MS = 1000

func gcsPositionRows(_ position: JSON?, _ accuracy: JSON?) -> [(String, String)]? {
    let coordinate = position?["value"].objectOrNil ?? position
    guard let coordinate, coordinate["valid"].bool else { return nil }
    let hdop = accuracy?["value"].double ?? .nan
    return [
        ("Latitude", String(format: "%.7f", coordinate["latitude"].double(.nan))),
        ("Longitude", String(format: "%.7f", coordinate["longitude"].double(.nan))),
        ("HDOP", hdop > 0 ? String(format: "%.1f m", hdop) : "N/A"),
    ]
}

private let GCS_POSITION_TITLE = "GCS position"

// Inside a NativeForm: value rows of the section it sits in (the ground station location block), the first one
// carrying the poll so it runs while the rows are there.
struct GcsPositionStatus: View {
    @State private var rows: [(String, String)]?
    @Environment(\.LocalNativeList) private var nativeList

    var body: some View {
        if nativeList {
            native
        } else {
            handDrawn
        }
    }

    @ViewBuilder private var native: some View {
        Group {
            if let first = rows?.first {
                LabeledContent(first.0, value: first.1)
            } else {
                LabeledContent(GCS_POSITION_TITLE, value: "Unavailable")
            }
        }
        .task { await poll() }
        if let shown = rows {
            ForEach(Array(shown.dropFirst()), id: \.0) { label, value in
                LabeledContent(label, value: value)
            }
        }
    }

    @MainActor private func poll() async {
        while !Task.isCancelled {
            rows = await offMain { gcsPositionRows(Qgc.get(GCS_POSITION), Qgc.get(GCS_ACCURACY)) }
            try? await Task.sleep(for: .milliseconds(GCS_POLL_MS))
        }
    }

    private var handDrawn: some View {
        VStack(alignment: .leading, spacing: 0) {
            if let shown = rows {
                SectionHeader(text: GCS_POSITION_TITLE)
                VStack(spacing: 6) {
                    ForEach(shown, id: \.0) { label, value in
                        HStack {
                            Text(label).font(.bodyMedium).frame(maxWidth: .infinity, alignment: .leading)
                            Text(value).font(.bodyMedium)
                        }
                    }
                }
                .padding(.horizontal, Space.s5)
                .padding(.vertical, Space.s2)
            }
        }
        .task { await poll() }
    }
}
