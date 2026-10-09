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

struct GcsPositionStatus: View {
    @State private var rows: [(String, String)]?

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if let shown = rows {
                SectionHeader(text: "GCS position")
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
        .task {
            while !Task.isCancelled {
                rows = await offMain { gcsPositionRows(Qgc.get(GCS_POSITION), Qgc.get(GCS_ACCURACY)) }
                try? await Task.sleep(for: .milliseconds(GCS_POLL_MS))
            }
        }
    }
}
