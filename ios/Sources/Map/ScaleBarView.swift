import SwiftUI

private let MAX_BAR_DP = 100.0

struct ScaleBarView: View {
    let latitude: Double
    let zoom: Double
    @State private var view: JSON?

    var body: some View {
        let across = metresAcross(latitude, zoom, MAX_BAR_DP)
        ZStack(alignment: .topLeading) {
            if let scale = mapScaleBar(view, MAX_BAR_DP) {
                VStack(alignment: .leading, spacing: 0) {
                    Text(scale.text)
                        .font(.labelSmall)
                        .foregroundStyle(.white)
                        .padding(.bottom, 2)
                    Rectangle()
                        .fill(.white.opacity(0.9))
                        .frame(maxWidth: scale.pixels)
                        .frame(height: 4)
                }
            }
        }
        .task(id: across) {
            guard let across else { return view = nil }
            view = await offMain { MapBridge.read("view.mapScale(\(across))") }
        }
    }
}
