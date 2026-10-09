import SwiftUI

let PALETTE_SETTING = "settings.appSettings.indoorPalette"
let PALETTE_INDOOR = 1
let PALETTE_OUTDOOR = 0
private let PALETTE_DEFAULTED = "paletteDefaultedToDark"

private let PALETTE_NAMES = ["\(PALETTE_INDOOR)": "Dark", "\(PALETTE_OUTDOOR)": "Light"]

func paletteNamed(_ fact: Fact) -> Fact {
    guard fact.path == PALETTE_SETTING, fact.enumValues.count == fact.enumStrings.count else { return fact }
    var named = fact
    named.enumStrings = zip(fact.enumValues, fact.enumStrings).map { raw, label in PALETTE_NAMES[raw] ?? label }
    return named
}

func paletteIsDark(_ value: Int?, _ systemDark: Bool) -> Bool {
    switch value {
    case PALETTE_INDOOR: true
    case PALETTE_OUTDOOR: false
    case nil: true
    default: systemDark
    }
}

private func changedFromDefault(_ control: JSON) -> Bool {
    control["changedFromDefault"].isNull || control["changedFromDefault"].bool
}

func shouldDefaultToDark(_ changedFromDefault: Bool, _ alreadyDefaulted: Bool) -> Bool {
    !changedFromDefault && !alreadyDefaulted
}

@MainActor
@Observable
final class PaletteDefault {
    static let shared = PaletteDefault()
    var awaiting = false
}

func paletteWindowStyle(_ followsSystem: Bool, _ dark: Bool) -> UIUserInterfaceStyle {
    followsSystem ? .unspecified : dark ? .dark : .light
}

@MainActor
private func applyPaletteWindowStyle(_ style: UIUserInterfaceStyle) {
    UIApplication.shared.connectedScenes
        .compactMap { $0 as? UIWindowScene }
        .flatMap(\.windows)
        .forEach { $0.overrideUserInterfaceStyle = style }
}

extension View {
    func windowStyle(_ style: UIUserInterfaceStyle) -> some View {
        onChange(of: style, initial: true) { _, now in applyPaletteWindowStyle(now) }
    }
}

@propertyWrapper
struct AppDarkTheme: DynamicProperty {
    @QgcPath(settingControl(PALETTE_SETTING)) private var control
    @Environment(\.colorScheme) private var scheme

    var wrappedValue: Bool {
        let value = control?["value"].int
        let systemDark = scheme == .dark
        return MainActor.assumeIsolated {
            let pending = PaletteDefault.shared.awaiting && value != PALETTE_INDOOR
            return paletteIsDark(pending ? PALETTE_INDOOR : value, systemDark)
        }
    }

    nonisolated func update() {
        MainActor.assumeIsolated {
            guard let read = control else { return }
            let awaited = PaletteDefault.shared
            if awaited.awaiting && read["value"].int == PALETTE_INDOOR { awaited.awaiting = false }
            let store = UserDefaults.standard
            guard !store.bool(forKey: PALETTE_DEFAULTED) else { return }
            if shouldDefaultToDark(changedFromDefault(read), false) {
                awaited.awaiting = true
                offMain {
                    guard !Qgc.set(PALETTE_SETTING, PALETTE_INDOOR) else { return }
                    onMain { awaited.awaiting = false }
                }
            }
            store.set(true, forKey: PALETTE_DEFAULTED)
        }
    }
}
