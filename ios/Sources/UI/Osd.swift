import SwiftUI

extension EnvironmentValues {
    @Entry var flyOsd = false
}

private let OSD_SHADOW_BLUR: CGFloat = 1.5
private let OSD_SHADOW_DROP: CGFloat = 0.5
private let OSD_SHADOW_OPACITY = 0.4

func osdBackdrop(_ color: Color, _ flyOsd: Bool) -> Color { flyOsd ? .clear : color }

func osdTint(_ color: Color, _ osdColor: Color, _ flyOsd: Bool) -> Color { flyOsd ? osdColor : color }

extension View {
    func osdShadow(_ on: Bool = true) -> some View {
        shadow(color: on ? .black.opacity(OSD_SHADOW_OPACITY) : .clear, radius: on ? OSD_SHADOW_BLUR : 0, x: 0, y: on ? OSD_SHADOW_DROP : 0)
    }
}
