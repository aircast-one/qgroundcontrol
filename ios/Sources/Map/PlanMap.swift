import SwiftUI

func planMapStyle() -> String {
    MapBridge.start()
    return qgcRasterStyle(currentMapType())
}

struct PlanMapScreen: View {
    var onCentre: ((Double, Double) -> Void)? = nil
    var itemPanel: ((Int, TrackPoint?, String?, (() -> Void)?) -> AnyView)? = nil
    var header: ((PlanBar) -> AnyView)? = nil
    var routeSettings: (() -> AnyView)? = nil
    var fitKey: Int = 0
    var onTemplates: (() -> Void)? = nil
    @State private var style: String?
    @Environment(\.theme) private var theme

    var body: some View {
        ZStack {
            theme.colors.surface
            if let style {
                PlanMapContent(
                    mapStyle: style,
                    onCentre: onCentre,
                    itemPanel: itemPanel,
                    header: header,
                    routeSettings: routeSettings,
                    fitKey: fitKey,
                    onTemplates: onTemplates
                )
            }
        }
        .task {
            guard style == nil else { return }
            style = await offMain { planMapStyle() }
        }
    }
}
