import SwiftUI

func planMapStyle() -> String {
    MapBridge.start()
    return qgcRasterStyle(currentMapType())
}

struct PlanMapScreen: View {
    var onClear: (() -> Void)? = nil
    var onCentre: ((Double, Double) -> Void)? = nil
    var itemEditor: ((Int, TrackPoint?, @escaping () -> Void, @escaping () -> Void) -> AnyView)? = nil
    var header: ((PlanUpload) -> AnyView)? = nil
    var fitKey: Int = 0
    var overlay: (() -> AnyView)? = nil
    var summaryHidden: Bool = false
    @State private var style: String?
    @Environment(\.theme) private var theme

    var body: some View {
        ZStack {
            theme.colors.surface
            if let style {
                PlanMapContent(
                    mapStyle: style,
                    onClear: onClear,
                    onCentre: onCentre,
                    itemEditor: itemEditor,
                    header: header,
                    fitKey: fitKey,
                    overlay: overlay,
                    summaryHidden: summaryHidden
                )
            }
        }
        .task {
            guard style == nil else { return }
            style = await offMain { planMapStyle() }
        }
    }
}
