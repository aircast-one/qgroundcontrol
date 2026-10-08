import MapLibre
import SwiftUI

struct AircastRoot: View {
    @QgcPath("view.flyState") private var flyState

    var body: some View {
        ZStack(alignment: .top) {
            StyleMap()
                .ignoresSafeArea()
            Text(flyState?["connected"].bool == true ? flyState?["mode"].string ?? "" : "No vehicle")
                .font(.titleMedium)
                .padding(Space.s3)
                .background(.thinMaterial, in: Capsule())
                .padding(.top, Space.s4)
        }
        .aircastTheme(dark: true)
    }
}

struct StyleMap: UIViewRepresentable {
    func makeUIView(context: Context) -> MLNMapView {
        let view = MLNMapView(frame: .zero)
        offMain {
            let url = qgcStyleURL(currentMapType())
            DispatchQueue.main.async { view.styleURL = url }
        }
        return view
    }

    func updateUIView(_ uiView: MLNMapView, context: Context) {}
}
