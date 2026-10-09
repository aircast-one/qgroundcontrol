import SwiftUI

extension EnvironmentValues {
    @Entry var flyOsd = false
}

private let OSD_SHADOW_BLUR: CGFloat = 3
private let OSD_SHADOW_DROP: CGFloat = 1

func osdBackdrop(_ color: Color, _ flyOsd: Bool) -> Color { flyOsd ? .clear : color }

func osdTint(_ color: Color, _ osdColor: Color, _ flyOsd: Bool) -> Color { flyOsd ? osdColor : color }

extension View {
    func osdShadow(_ on: Bool = true) -> some View {
        let color: Color = on ? .black : .clear
        let radius = on ? OSD_SHADOW_BLUR : 0
        let drop = on ? OSD_SHADOW_DROP : 0
        return shadow(color: color, radius: radius, x: 0, y: drop)
            .shadow(color: color, radius: radius, x: 0, y: drop)
    }
}
