import SwiftUI

// Whether the shell lays out for a phone held upright. A Mac window is resized freely and its minimum size keeps it
// landscape-shaped, so the Mac always takes the landscape layout rather than flipping to the phone one when a
// window is dragged taller than it is wide.
func shellIsPortrait(_ root: CGSize) -> Bool {
    #if os(macOS)
    return false
    #else
    return root.height > root.width
    #endif
}

#if os(macOS)
let MAC_WINDOW_DEFAULT_WIDTH: CGFloat = 1280
let MAC_WINDOW_DEFAULT_HEIGHT: CGFloat = 800
let MAC_WINDOW_MIN_WIDTH: CGFloat = 1024
let MAC_WINDOW_MIN_HEIGHT: CGFloat = 640

// The Fly tab's sheet of every flight action (REQUESTABLE_SHEETS).
let FLIGHT_ACTIONS_SHEET = "more"

// The flight actions the Vehicle menu offers, in menu order, with the name shown before the core has titled them.
// Each is a guided offer the Fly tab's deck already carries, so a menu pick runs the deck's own confirmation.
let MENU_FLIGHT_ACTIONS: [(id: String, fallback: String)] = [("rtl", "Return"), ("land", "Land"), (PAUSE, "Pause")]

struct MenuFlightItem: Identifiable, Equatable {
    let id: String
    let title: String
    let enabled: Bool
}

// The menu shows every action at all times, so the menu does not reshuffle as offers come and go; one the vehicle
// does not offer right now is greyed out. A title ends in an ellipsis when the pick opens a confirmation or a value
// to set rather than acting at once, as Mac menus mark items that ask for more.
func menuFlightItems(_ offers: [String: GuidedOffer], _ connected: Bool) -> [MenuFlightItem] {
    MENU_FLIGHT_ACTIONS.map { action in
        let offer = offers[action.id]
        let title = offer?.title.nonBlank ?? action.fallback
        let asks = action.id == PAUSE || offer?.confirm != false
        return MenuFlightItem(id: action.id, title: asks ? "\(title)\u{2026}" : title, enabled: connected && offer?.shown == true && offer?.ready == true)
    }
}

// ⌘1 … ⌘9 for the first nine tabs, in the order the shell shows them.
func tabShortcutKey(_ index: Int) -> Character? {
    guard (0..<9).contains(index) else { return nil }
    return Character(String(index + 1))
}

enum ShellEscape: Equatable {
    case CloseSettings
    case LeaveFullScreen
}

// What Esc closes in the main window. Settings is drawn over the window rather than presented as a sheet, and
// full-screen video is the shell's own mode, so neither gets the Esc a Mac sheet answers to on its own.
func shellEscape(settingsOpen: Bool, fullScreen: Bool) -> ShellEscape? {
    if settingsOpen { return .CloseSettings }
    if fullScreen { return .LeaveFullScreen }
    return nil
}

// What the shell lends the menu bar while its window is the active one.
struct ShellMenu {
    let tabs: [Tab]
    let current: Tab?
    let connected: Bool
    let flight: [MenuFlightItem]
    let select: (Tab) -> Void
    let openSettings: () -> Void
    let requestFlight: (String) -> Void
    let openFlightActions: () -> Void
}

// What the Plan tab lends the File menu while it is showing.
struct PlanFileMenu {
    let canOpen: Bool
    let canSave: Bool
    let open: () -> Void
    let save: () -> Void
    let saveAs: () -> Void
}

private struct ShellMenuKey: FocusedValueKey {
    typealias Value = ShellMenu
}

private struct PlanFileMenuKey: FocusedValueKey {
    typealias Value = PlanFileMenu
}

extension FocusedValues {
    var shellMenu: ShellMenu? {
        get { self[ShellMenuKey.self] }
        set { self[ShellMenuKey.self] = newValue }
    }

    var planFileMenu: PlanFileMenu? {
        get { self[PlanFileMenuKey.self] }
        set { self[PlanFileMenuKey.self] = newValue }
    }
}

extension View {
    func planFileMenu(_ menu: PlanFileMenu) -> some View { focusedSceneValue(\.planFileMenu, menu) }
}

struct MacMenuCommands: Commands {
    @FocusedValue(\.shellMenu) private var shell
    @FocusedValue(\.planFileMenu) private var plan

    var body: some Commands {
        CommandGroup(replacing: .appSettings) {
            Button("Settings\u{2026}") { shell?.openSettings() }
                .keyboardShortcut(",", modifiers: .command)
                .disabled(shell == nil)
        }
        // The app drives one vehicle session, so it keeps to the one window: File > New Window gives way to the plan.
        // Saving sits here too, as replacing .saveItem would also take away File > Close.
        CommandGroup(replacing: .newItem) {
            Button("Open Plan\u{2026}") { plan?.open() }
                .keyboardShortcut("o", modifiers: .command)
                .disabled(plan?.canOpen != true)
            Button("Save Plan") { plan?.save() }
                .keyboardShortcut("s", modifiers: .command)
                .disabled(plan?.canSave != true)
            Button("Save Plan As\u{2026}") { plan?.saveAs() }
                .keyboardShortcut("s", modifiers: [.command, .shift])
                .disabled(plan?.canSave != true)
        }
        CommandGroup(before: .toolbar) {
            let tabs = shell?.tabs ?? visibleTabs(false)
            ForEach(Array(tabs.enumerated()), id: \.element) { index, entry in
                Toggle(entry.label, isOn: Binding(get: { shell?.current == entry }, set: { _ in shell?.select(entry) }))
                    .keyboardShortcut(tabShortcutKey(index).map { KeyboardShortcut(KeyEquivalent($0), modifiers: .command) })
                    .disabled(shell == nil)
            }
            Divider()
        }
        CommandMenu("Vehicle") {
            ForEach(shell?.flight ?? menuFlightItems([:], false)) { item in
                Button(item.title) { shell?.requestFlight(item.id) }
                    .disabled(shell == nil || !item.enabled)
            }
            Divider()
            Button("Flight Actions\u{2026}") { shell?.openFlightActions() }
                .disabled(shell?.connected != true)
        }
    }
}

// The Mac window's chrome for the shell: the tabs as a segmented control in the window toolbar (in place of the
// iPad's navigation rail), Settings beside them, Esc for what the shell draws itself, and the menu bar's hooks.
struct MacShellChrome: ViewModifier {
    let tabs: [Tab]
    let current: Tab?
    let fullScreen: Bool
    let escape: ShellEscape?
    let onSelect: (Tab) -> Void
    let onSettings: () -> Void
    let onEscape: (ShellEscape) -> Void
    let onFlightAction: (String) -> Void
    let onFlightActions: () -> Void
    @QgcPath(GUIDED_ACTIONS) private var actionsJson
    @QgcPath(FLY_STATE) private var flyJson

    init(
        tabs: [Tab],
        current: Tab?,
        fullScreen: Bool,
        escape: ShellEscape?,
        onSelect: @escaping (Tab) -> Void,
        onSettings: @escaping () -> Void,
        onEscape: @escaping (ShellEscape) -> Void,
        onFlightAction: @escaping (String) -> Void,
        onFlightActions: @escaping () -> Void
    ) {
        self.tabs = tabs
        self.current = current
        self.fullScreen = fullScreen
        self.escape = escape
        self.onSelect = onSelect
        self.onSettings = onSettings
        self.onEscape = onEscape
        self.onFlightAction = onFlightAction
        self.onFlightActions = onFlightActions
    }

    private var menu: ShellMenu {
        let connected = flyState(flyJson)?.connected == true
        return ShellMenu(
            tabs: tabs,
            current: current,
            connected: connected,
            flight: menuFlightItems(guidedOffers(actionsJson), connected),
            select: onSelect,
            openSettings: onSettings,
            requestFlight: onFlightAction,
            openFlightActions: onFlightActions
        )
    }

    private var selection: Binding<Tab?> {
        Binding(get: { current }, set: { picked in if let picked { onSelect(picked) } })
    }

    func body(content: Content) -> some View {
        content
            .toolbar {
                ToolbarItem(placement: .principal) {
                    Picker("Section", selection: selection) {
                        ForEach(tabs, id: \.self) { entry in
                            Text(entry.label).tag(Optional(entry))
                        }
                    }
                    .pickerStyle(.segmented)
                    .labelsHidden()
                }
                ToolbarItem(placement: .primaryAction) {
                    Button(action: onSettings) {
                        Label("Settings", systemImage: Icon.settings.rawValue)
                    }
                    .help("Settings")
                }
            }
            .toolbar(fullScreen ? .hidden : .visible, for: .windowToolbar)
            .background { escapeKey }
            .focusedSceneValue(\.shellMenu, menu)
    }

    // A button no one sees or clicks, there only for its Esc shortcut while there is something for Esc to close.
    @ViewBuilder
    private var escapeKey: some View {
        if let escape {
            Button("Close") { onEscape(escape) }
                .keyboardShortcut(.cancelAction)
                .buttonStyle(.plain)
                .opacity(0)
                .frame(width: 0, height: 0)
                .accessibilityHidden(true)
        }
    }
}

extension View {
    func macShellChrome(_ chrome: MacShellChrome) -> some View { modifier(chrome) }
}
#endif
