import SwiftUI

let DISABLED_ALPHA = 0.38

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
        colors: .aircastDark,
        aircast: AircastColors(
            success: AircastTokens.Dark.success, onSuccess: AircastTokens.Dark.onSuccess, successContainer: AircastTokens.Dark.successContainer,
            warning: AircastTokens.Dark.warning, onWarning: AircastTokens.Dark.onWarning, warningContainer: AircastTokens.Dark.warningContainer,
            mission: Color(hex: 0xFFB870), alert: Color(hex: 0xFFB870),
            mapLand: Color(hex: 0x1B2A24), mapWater: Color(hex: 0x12263A), mapRoad: Color(hex: 0x3A4A44),
            outdoorBackground: Color(hex: 0x000000), outdoorForeground: Color(hex: 0xFFFFFF), outdoorAccent: Color(hex: 0xFFD60A), onOutdoorAccent: Color(hex: 0x000000)
        )
    )

    static let lightTheme = Theme(
        dark: false,
        colors: .aircastLight,
        aircast: AircastColors(
            success: AircastTokens.Light.success, onSuccess: AircastTokens.Light.onSuccess, successContainer: AircastTokens.Light.successContainer,
            warning: AircastTokens.Light.warning, onWarning: AircastTokens.Light.onWarning, warningContainer: AircastTokens.Light.warningContainer,
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

struct FilledButtonStyle: PrimitiveButtonStyle {
    @Environment(\.theme) private var theme
    @Environment(\.isEnabled) private var isEnabled

    func makeBody(configuration: Configuration) -> some View {
        Button(role: configuration.role, action: configuration.trigger) {
            configuration.label.foregroundStyle(isEnabled ? theme.colors.onPrimary : theme.colors.onSurface.opacity(DISABLED_ALPHA))
        }
        .buttonStyle(.borderedProminent)
        .tint(theme.colors.primary)
    }
}

extension PrimitiveButtonStyle where Self == FilledButtonStyle {
    static var filled: FilledButtonStyle { FilledButtonStyle() }
}

struct TextButtonStyle: PrimitiveButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        Button(role: configuration.role, action: configuration.trigger) {
            configuration.label
                .frame(minWidth: MINIMUM_TOUCH_TARGET, minHeight: MINIMUM_TOUCH_TARGET)
                .contentShape(Rectangle())
        }
        .buttonStyle(.borderless)
    }
}

extension PrimitiveButtonStyle where Self == TextButtonStyle {
    static var text: TextButtonStyle { TextButtonStyle() }
}

extension View {
    func font(_ scale: TypeScale) -> some View { modifier(ScaledType(scale)) }

    func aircastTheme(dark: Bool) -> some View {
        environment(\.theme, dark ? .darkTheme : .lightTheme)
            .tint((dark ? Theme.darkTheme : Theme.lightTheme).colors.primary)
    }
}

let MINIMUM_TOUCH_TARGET: CGFloat = 48

struct MinimumTouchTarget: Shape {
    func path(in rect: CGRect) -> Path {
        Path(rect.insetBy(dx: min(0, (rect.width - MINIMUM_TOUCH_TARGET) / 2), dy: min(0, (rect.height - MINIMUM_TOUCH_TARGET) / 2)))
    }
}

extension View {
    func minimumTouchTarget() -> some View { contentShape(MinimumTouchTarget()) }
}
