import SwiftUI

extension EnvironmentValues {
    @Entry var flyOsd = false
}

private let OSD_SHADOW_BLUR: CGFloat = 3
private let OSD_SHADOW_DROP: CGFloat = 1

func osdBackdrop(_ color: Color, _ flyOsd: Bool) -> Color { flyOsd ? .clear : color }

func osdTint(_ color: Color, _ osdColor: Color, _ flyOsd: Bool) -> Color { flyOsd ? osdColor : color }

extension View {
    func osdShadow() -> some View {
        shadow(color: .black, radius: OSD_SHADOW_BLUR, x: 0, y: OSD_SHADOW_DROP)
            .shadow(color: .black, radius: OSD_SHADOW_BLUR, x: 0, y: OSD_SHADOW_DROP)
    }
}
