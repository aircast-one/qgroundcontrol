import SwiftUI

extension Color {
    init(hex: UInt32) {
        self.init(.sRGB,
                  red: Double((hex >> 16) & 0xFF) / 255,
                  green: Double((hex >> 8) & 0xFF) / 255,
                  blue: Double(hex & 0xFF) / 255,
                  opacity: 1)
    }
}

struct MaterialColors {
    let primary, onPrimary, primaryContainer, onPrimaryContainer: Color
    let secondary, onSecondary, secondaryContainer, onSecondaryContainer: Color
    let tertiary, onTertiary, tertiaryContainer: Color
    let error, onError, errorContainer, onErrorContainer: Color
    let background, onBackground, surface, onSurface, surfaceVariant, onSurfaceVariant: Color
    let outline, outlineVariant: Color
    let surfaceDim, surfaceBright: Color
    let surfaceContainerLowest, surfaceContainerLow, surfaceContainer, surfaceContainerHigh, surfaceContainerHighest: Color
    let inverseSurface, inverseOnSurface, scrim: Color
}

struct AircastColors {
    let success, onSuccess, successContainer: Color
    let warning, onWarning, warningContainer: Color
    let mission, alert: Color
    let mapLand, mapWater, mapRoad: Color
    let outdoorBackground, outdoorForeground, outdoorAccent, onOutdoorAccent: Color
}

struct Theme {
    let dark: Bool
    let colors: MaterialColors
    let aircast: AircastColors

    static let darkTheme = Theme(
        dark: true,
        colors: MaterialColors(
            primary: Color(hex: 0xA9C7FF), onPrimary: Color(hex: 0x07305F), primaryContainer: Color(hex: 0x254777), onPrimaryContainer: Color(hex: 0xD6E3FF),
            secondary: Color(hex: 0xBCC7DC), onSecondary: Color(hex: 0x273141), secondaryContainer: Color(hex: 0x3D4758), onSecondaryContainer: Color(hex: 0xD8E3F8),
            tertiary: Color(hex: 0xDBBCE1), onTertiary: Color(hex: 0x3E2845), tertiaryContainer: Color(hex: 0x563E5C),
            error: Color(hex: 0xFFB4AB), onError: Color(hex: 0x690005), errorContainer: Color(hex: 0x93000A), onErrorContainer: Color(hex: 0xFFDAD6),
            background: Color(hex: 0x111318), onBackground: Color(hex: 0xE2E2E9), surface: Color(hex: 0x111318), onSurface: Color(hex: 0xE2E2E9),
            surfaceVariant: Color(hex: 0x44474E), onSurfaceVariant: Color(hex: 0xC4C6D0),
            outline: Color(hex: 0x8E9099), outlineVariant: Color(hex: 0x44474E),
            surfaceDim: Color(hex: 0x111318), surfaceBright: Color(hex: 0x37393E),
            surfaceContainerLowest: Color(hex: 0x0C0E13), surfaceContainerLow: Color(hex: 0x191C20), surfaceContainer: Color(hex: 0x1D2024),
            surfaceContainerHigh: Color(hex: 0x282A2F), surfaceContainerHighest: Color(hex: 0x33353A),
            inverseSurface: Color(hex: 0xE2E2E9), inverseOnSurface: Color(hex: 0x2E3036), scrim: Color(hex: 0x000000)
        ),
        aircast: AircastColors(
            success: Color(hex: 0x7BDA8F), onSuccess: Color(hex: 0x00391A), successContainer: Color(hex: 0x005229),
            warning: Color(hex: 0xF2C06B), onWarning: Color(hex: 0x422C00), warningContainer: Color(hex: 0x5F4100),
            mission: Color(hex: 0xFFB870), alert: Color(hex: 0xFFB870),
            mapLand: Color(hex: 0x1B2A24), mapWater: Color(hex: 0x12263A), mapRoad: Color(hex: 0x3A4A44),
            outdoorBackground: Color(hex: 0x000000), outdoorForeground: Color(hex: 0xFFFFFF), outdoorAccent: Color(hex: 0xFFD60A), onOutdoorAccent: Color(hex: 0x000000)
        )
    )

    static let lightTheme = Theme(
        dark: false,
        colors: MaterialColors(
            primary: Color(hex: 0x3A5F93), onPrimary: Color(hex: 0xFFFFFF), primaryContainer: Color(hex: 0xD6E3FF), onPrimaryContainer: Color(hex: 0x001B3E),
            secondary: Color(hex: 0x555F71), onSecondary: Color(hex: 0xFFFFFF), secondaryContainer: Color(hex: 0xD8E3F8), onSecondaryContainer: Color(hex: 0x121C2B),
            tertiary: Color(hex: 0x6E5676), onTertiary: Color(hex: 0xFFFFFF), tertiaryContainer: Color(hex: 0xF7D8FF),
            error: Color(hex: 0xBA1A1A), onError: Color(hex: 0xFFFFFF), errorContainer: Color(hex: 0xFFDAD6), onErrorContainer: Color(hex: 0x410002),
            background: Color(hex: 0xF9F9FF), onBackground: Color(hex: 0x191C20), surface: Color(hex: 0xF9F9FF), onSurface: Color(hex: 0x191C20),
            surfaceVariant: Color(hex: 0xE2E2E9), onSurfaceVariant: Color(hex: 0x44474E),
            outline: Color(hex: 0x74777F), outlineVariant: Color(hex: 0xC4C6D0),
            surfaceDim: Color(hex: 0xD9D9E0), surfaceBright: Color(hex: 0xF9F9FF),
            surfaceContainerLowest: Color(hex: 0xFFFFFF), surfaceContainerLow: Color(hex: 0xF3F3FA), surfaceContainer: Color(hex: 0xEDEDF4),
            surfaceContainerHigh: Color(hex: 0xE7E8EE), surfaceContainerHighest: Color(hex: 0xE2E2E9),
            inverseSurface: Color(hex: 0x2E3036), inverseOnSurface: Color(hex: 0xF0F0F7), scrim: Color(hex: 0x000000)
        ),
        aircast: AircastColors(
            success: Color(hex: 0x1B6D36), onSuccess: Color(hex: 0xFFFFFF), successContainer: Color(hex: 0xA6F5B5),
            warning: Color(hex: 0x7C5800), onWarning: Color(hex: 0xFFFFFF), warningContainer: Color(hex: 0xFFDEA6),
            mission: Color(hex: 0xB45F00), alert: Color(hex: 0xB45F00),
            mapLand: Color(hex: 0xDDE8DC), mapWater: Color(hex: 0xB9D7F0), mapRoad: Color(hex: 0xFFFFFF),
            outdoorBackground: Color(hex: 0x000000), outdoorForeground: Color(hex: 0xFFFFFF), outdoorAccent: Color(hex: 0xFFD60A), onOutdoorAccent: Color(hex: 0x000000)
        )
    )
}

extension EnvironmentValues {
    @Entry var theme: Theme = .darkTheme
}

enum Space {
    static let s1: CGFloat = 4
    static let s2: CGFloat = 8
    static let s3: CGFloat = 12
    static let s4: CGFloat = 16
    static let s5: CGFloat = 20
    static let s6: CGFloat = 24
    static let s8: CGFloat = 32
}

enum Corner {
    static let extraSmall: CGFloat = 4
    static let small: CGFloat = 8
    static let medium: CGFloat = 12
    static let large: CGFloat = 16
    static let extraLarge: CGFloat = 28
}

enum TypeScale {
    case displayLarge, displayMedium, displaySmall
    case headlineLarge, headlineMedium, headlineSmall
    case titleLarge, titleMedium, titleSmall
    case bodyLarge, bodyMedium, bodySmall
    case labelLarge, labelMedium, labelSmall
    case telemetry, telemetryMedium

    var size: CGFloat {
        switch self {
        case .displayLarge: 57
        case .displayMedium: 45
        case .displaySmall: 36
        case .headlineLarge: 32
        case .headlineMedium: 28
        case .headlineSmall: 24
        case .titleLarge: 22
        case .titleMedium: 16
        case .titleSmall: 14
        case .bodyLarge: 16
        case .bodyMedium: 14
        case .bodySmall: 12
        case .labelLarge: 14
        case .labelMedium: 12
        case .labelSmall: 11
        case .telemetry: 22
        case .telemetryMedium: 16
        }
    }

    var weight: Font.Weight {
        switch self {
        case .titleMedium, .titleSmall, .labelLarge, .labelMedium, .labelSmall, .telemetry, .telemetryMedium: .medium
        default: .regular
        }
    }

    func font(_ size: CGFloat) -> Font {
        let font = Font.system(size: size, weight: weight)
        return self == .telemetry || self == .telemetryMedium ? font.monospacedDigit() : font
    }

    var font: Font { font(size) }
}

private struct ScaledType: ViewModifier {
    let scale: TypeScale
    @ScaledMetric private var size: CGFloat

    init(_ scale: TypeScale) {
        self.scale = scale
        _size = ScaledMetric(wrappedValue: scale.size, relativeTo: .body)
    }

    func body(content: Content) -> some View { content.font(scale.font(size)) }
}

struct AircastTheme<Content: View>: View {
    var dark: Bool? = nil
    @ViewBuilder let content: () -> Content

    var body: some View { content().aircastTheme(dark: dark ?? SystemAppearance.shared.dark) }
}

extension View {
    func font(_ scale: TypeScale) -> some View { modifier(ScaledType(scale)) }

    func aircastTheme(dark: Bool) -> some View {
        environment(\.theme, dark ? .darkTheme : .lightTheme)
            .preferredColorScheme(dark ? .dark : .light)
            .tint((dark ? Theme.darkTheme : Theme.lightTheme).colors.primary)
    }
}
