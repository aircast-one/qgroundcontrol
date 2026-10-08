import SwiftUI
import UIKit

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
private enum PaletteDefault {
    static var awaiting = false
}

@MainActor
@Observable
final class SystemAppearance {
    static let shared = SystemAppearance()
    private(set) var dark: Bool

    private init() {
        let scene = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.first
        dark = scene?.traitCollection.userInterfaceStyle == .dark
        _ = scene?.registerForTraitChanges([UITraitUserInterfaceStyle.self]) { [weak self] (changed: UIWindowScene, _: UITraitCollection) in
            self?.dark = changed.traitCollection.userInterfaceStyle == .dark
        }
    }
}

@propertyWrapper
struct AppDarkTheme: DynamicProperty {
    @QgcPath(settingControl(PALETTE_SETTING)) private var control

    var wrappedValue: Bool {
        let value = control?["value"].int
        return MainActor.assumeIsolated {
            let pending = PaletteDefault.awaiting && value != PALETTE_INDOOR
            return paletteIsDark(pending ? PALETTE_INDOOR : value, SystemAppearance.shared.dark)
        }
    }

    nonisolated func update() {
        MainActor.assumeIsolated {
            guard let read = control else { return }
            if PaletteDefault.awaiting && read["value"].int == PALETTE_INDOOR { PaletteDefault.awaiting = false }
            let store = UserDefaults.standard
            guard !store.bool(forKey: PALETTE_DEFAULTED) else { return }
            if shouldDefaultToDark(changedFromDefault(read), false) {
                PaletteDefault.awaiting = true
                offMain {
                    guard !Qgc.set(PALETTE_SETTING, PALETTE_INDOOR) else { return }
                    onMain { PaletteDefault.awaiting = false }
                }
            }
            store.set(true, forKey: PALETTE_DEFAULTED)
        }
    }
}
