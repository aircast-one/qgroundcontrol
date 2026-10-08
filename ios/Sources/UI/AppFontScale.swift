import SwiftUI

private let APP_FONT_POINT_SIZE = "settings.appSettings.appFontPointSize"
private let PLATFORM_FONT_POINT_SIZE: Float = 14
private let FONT_POINT_SIZE_MIN: Float = 6
private let FONT_POINT_SIZE_MAX: Float = 48
private let FONT_SIZE_POLL_MS = 1000

private let BODY_POINTS: [(size: DynamicTypeSize, points: Double)] = [
    (.xSmall, 14), (.small, 15), (.medium, 16), (.large, 17), (.xLarge, 19), (.xxLarge, 21), (.xxxLarge, 23),
    (.accessibility1, 28), (.accessibility2, 33), (.accessibility3, 40), (.accessibility4, 47), (.accessibility5, 53),
]

func appFontScale(_ pointSize: Double?) -> Float {
    guard let size = pointSize.map(Float.init), (FONT_POINT_SIZE_MIN...FONT_POINT_SIZE_MAX).contains(size) else { return 1 }
    return size / PLATFORM_FONT_POINT_SIZE
}

private func pointSize(_ fact: JSON) -> Double? {
    guard case .number(let points) = fact["value"] else { return nil }
    return points
}

private func scaledTypeSize(_ system: DynamicTypeSize, _ scale: Float) -> DynamicTypeSize {
    let target = (BODY_POINTS.first { $0.size == system }?.points ?? 17) * Double(scale)
    return BODY_POINTS.min { abs($0.points - target) < abs($1.points - target) }?.size ?? system
}

struct AppFontScale<Content: View>: View {
    @ViewBuilder let content: () -> Content
    @State private var scale: Float = 1
    @Environment(\.dynamicTypeSize) private var system

    var body: some View {
        content()
            .dynamicTypeSize(scaledTypeSize(system, scale))
            .task {
                while !Task.isCancelled {
                    scale = await offMain { appFontScale(pointSize(Qgc.get(APP_FONT_POINT_SIZE))) }
                    try? await Task.sleep(for: .milliseconds(FONT_SIZE_POLL_MS))
                }
            }
    }
}
