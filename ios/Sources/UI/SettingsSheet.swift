import SwiftUI

func sheetDrilled(_ openPage: String?, _ setupOpen: Bool) -> Bool { openPage != nil || setupOpen }

func openingGroup(_ requested: String?) -> SettingsGroup { requested.map { pageLook($0).group } ?? .General }

private let SETTINGS_SHEET_LAYER: Double = 10
private let SETTINGS_PANEL_ALPHA = 0.9

#if os(iOS)
private let SETTINGS_SEARCH_PLACEMENT = SearchFieldPlacement.navigationBarDrawer(displayMode: .always)
#else
private let SETTINGS_SEARCH_PLACEMENT = SearchFieldPlacement.automatic
#endif

// One page pushed above the tabs in the Settings NavigationStack: a settings page by title, aircraft setup, or a
// device page by href (the Device tab's own stack).
enum SettingsRoute: Hashable {
    case Page(String)
    case Setup
    case Device(String)
}

@MainActor
private func settingsChangeNotice(_ host: SnackbarHostState, _ scope: ViewScope) -> ChangeNotice {
    ChangeNotice { message, undo in
        scope.launch {
            host.currentSnackbarData?.dismiss()
            let result = await host.showSnackbar(message, actionLabel: undo == nil ? nil : "Undo", duration: .Long)
            guard result == .ActionPerformed, let undo, let refused = await undo() else { return }
            await host.showSnackbar("Undo failed: \(refused)")
        }
    }
}

private func shownOnOpen(_ page: String?, _ group: SettingsGroup) -> String? {
    page.flatMap { pageLook($0).group == group && !pageLook($0).inline ? $0 : nil }
}

// The stack a requested page opens: its page pushed above its tab, or nothing for a page the tab draws inline.
func requestedRoutes(_ page: String?, _ group: SettingsGroup) -> [SettingsRoute] {
    shownOnOpen(page, group).map { [.Page($0)] } ?? []
}

func setupShowing(_ path: [SettingsRoute]) -> Bool { path.contains(.Setup) }

func deviceHrefs(_ path: [SettingsRoute]) -> [String] {
    path.compactMap { route -> String? in
        guard case .Device(let href) = route else { return nil }
        return href
    }
}

// The path with its device pages replaced by hrefs, in order, on top.
func withDeviceHrefs(_ path: [SettingsRoute], _ hrefs: [String]) -> [SettingsRoute] {
    path.filter { route in
        guard case .Device = route else { return true }
        return false
    } + hrefs.map { SettingsRoute.Device($0) }
}

// Where an open from a tab goes: the Device tab opens device pages, every other tab settings pages.
func tabRoute(_ group: SettingsGroup, _ opened: String) -> SettingsRoute { group == .Device ? .Device(opened) : .Page(opened) }

// Settings: a NavigationStack over the tabs (General, Safety … Device) with search in the navigation bar. Every
// drill-in — a page, aircraft setup and the pages inside it, a device page — is a push onto this one stack, so it
// gets the system back button and swipe-back; deep links and programmatic opens (navigation.settingsPage,
// openAircraft, addLinkRequested) set the stack.
struct SettingsSheet: View {
    let requested: String?
    let onClose: () -> Void
    @Environment(AppNavigationState.self) private var navigation
    @Environment(\.theme) private var theme
    @FlyIsPortrait private var portrait
    @SettingsPages private var everyPage
    @State private var group: SettingsGroup
    @State private var pager: SettingsGroup?
    @State private var page: String?
    @State private var path: [SettingsRoute]
    @State private var enteredForSetup = false
    @State private var query: String?
    @State private var snackbars: SnackbarHostState
    @State private var noticeScope: ViewScope
    @State private var notice: ChangeNotice

    init(requested: String?, onClose: @escaping () -> Void) {
        self.requested = requested
        self.onClose = onClose
        let opening = openingGroup(requested)
        _group = State(initialValue: opening)
        _pager = State(initialValue: opening)
        _page = State(initialValue: requested)
        _path = State(initialValue: requestedRoutes(requested, opening))
        let host = SnackbarHostState()
        let scope = ViewScope()
        _snackbars = State(initialValue: host)
        _noticeScope = State(initialValue: scope)
        _notice = State(initialValue: settingsChangeNotice(host, scope))
    }

    private var searchText: Binding<String> {
        Binding(get: { query ?? "" }, set: { typed in query = typed })
    }

    private var searching: Binding<Bool> {
        Binding(get: { query != nil }, set: { shown in query = shown ? query ?? "" : nil })
    }

    private func openSetup(_ component: String?) {
        navigation.setupPage = component
        if let at = path.firstIndex(of: .Setup) {
            path = Array(path[...at])
        } else {
            path.append(.Setup)
        }
    }

    private func openFromSearch(_ title: String) {
        if pageLook(title).inline {
            group = pageLook(title).group
            page = title
            query = nil
            path = []
        } else {
            path.append(.Page(title))
        }
    }

    private func pickTab(_ entry: SettingsGroup) {
        group = entry
        page = nil
        path = []
    }

    var body: some View {
        NavigationStack(path: $path) {
            root
                .navigationTitle("Settings")
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .confirmationAction) {
                        Button("Done", action: onClose)
                    }
                }
                .searchable(text: searchText, isPresented: searching, placement: SETTINGS_SEARCH_PLACEMENT, prompt: "Search settings")
                .navigationDestination(for: SettingsRoute.self) { route in destination(route) }
        }
        .overlay(alignment: .bottom) {
            SnackbarHost(hostState: snackbars) { AppSnackbar(data: $0) }
        }
        .environment(\.LocalChangeNotice, notice)
        .background {
            theme.colors.surface
                .opacity(portrait ? 1 : SETTINGS_PANEL_ALPHA)
                .ignoresSafeArea()
                .contentShape(Rectangle())
                .onTapGesture {}
        }
        .zIndex(SETTINGS_SHEET_LAYER)
        .onChange(of: path) { before, now in
            guard setupShowing(before) && !setupShowing(now) else { return }
            let closes = enteredForSetup
            enteredForSetup = false
            if closes { onClose() }
        }
        .onChange(of: navigation.aircraftRequested, initial: true) { before, asked in
            guard asked else { return }
            let openedForAircraft = before == asked
            if openedForAircraft {
                group = .General
                enteredForSetup = true
            }
            query = nil
            if path.first != .Setup { path = [.Setup] }
            navigation.aircraftRequested = false
        }
        .onChange(of: navigation.settingsPage, initial: true) { _, asked in
            guard let asked else { return }
            enteredForSetup = false
            group = pageLook(asked).group
            page = asked
            query = nil
            path = requestedRoutes(asked, pageLook(asked).group)
            navigation.settingsPage = nil
        }
        .onChange(of: setupShowing(path) ? nil : group, initial: true) { _, showing in navigation.settingsShowing = showing }
        .onDisappear {
            navigation.settingsShowing = nil
            noticeScope.cancel()
        }
    }

    @ViewBuilder
    private var root: some View {
        if let typed = query {
            SettingsSearch(query: typed, onOpenSetup: openSetup, onOpen: openFromSearch)
        } else {
            VStack(spacing: 0) {
                SheetTabs(selected: pager ?? group, edgePadding: Space.s2) { entry in withAnimation { pager = entry } }
                    .frame(maxWidth: .infinity, alignment: .leading)
                Divider()
                SettingsPager(pager: $pager, group: group, swipeable: true, onSettled: pickTab) { shown in
                    SettingsScreen(
                        group: shown,
                        everyPage: everyPage,
                        onOpen: { opened in path.append(tabRoute(shown, opened)) },
                        onOpenSetup: openSetup
                    )
                    .id(shown == group ? page : nil)
                }
            }
        }
    }

    @ViewBuilder
    private func destination(_ route: SettingsRoute) -> some View {
        switch route {
        case .Page(let title):
            Group {
                if let entry = everyPage.first(where: { $0.title == title }) {
                    SettingsPageScreen(page: entry)
                } else {
                    Text("Reading settings.").padding(16).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                }
            }
            .nativePage(pageTitle(title))
        case .Setup:
            SetupScreen().nativePage(AIRCRAFT_SETUP)
        case .Device(let href):
            DevicePageScreen(
                href: href,
                onOpen: { next in path.append(.Device(next)) },
                onOpenFrom: { next, returnTo in path = withDeviceHrefs(path, deviceRoutesOpenedFrom(deviceHrefs(path), next, returnTo)) },
                onClose: { if !path.isEmpty { path.removeLast() } }
            )
        }
    }
}

private struct SettingsPager<Content: View>: View {
    @Binding var pager: SettingsGroup?
    let group: SettingsGroup
    let swipeable: Bool
    let onSettled: (SettingsGroup) -> Void
    @ViewBuilder let content: (SettingsGroup) -> Content
    @State private var scrolling = false

    private func settle(_ now: SettingsGroup?) {
        guard !scrolling, let now, now != group else { return }
        onSettled(now)
    }

    var body: some View {
        ScrollViewReader { proxy in
            ScrollView(.horizontal) {
                LazyHStack(spacing: 0) {
                    ForEach(SettingsGroup.allCases, id: \.self) { entry in
                        content(entry)
                            .environment(\.isScrollEnabled, true)
                            .containerRelativeFrame(.horizontal)
                            .id(entry)
                    }
                }
                .scrollTargetLayout()
            }
            .scrollTargetBehavior(.viewAligned(limitBehavior: .always))
            .mask { Rectangle().ignoresSafeArea(.container, edges: .vertical) }
            .scrollIndicators(.hidden)
            .scrollPosition(id: $pager)
            .environment(\.isScrollEnabled, swipeable)
            .onChange(of: group, initial: true) { _, now in if pager != now { pager = now } }
            .onChange(of: pager) { _, now in settle(now) }
            .modifier(ScrollSettle { moving in
                scrolling = moving
                settle(pager)
            })
            .onGeometryChange(for: CGFloat.self) { $0.size.width } action: { _ in
                pager = group
                proxy.scrollTo(group)
            }
        }
    }
}

private struct ScrollSettle: ViewModifier {
    let onMoving: (Bool) -> Void

    func body(content: Content) -> some View {
        if #available(iOS 18, macOS 15, *) {
            content.onScrollPhaseChange { _, phase in onMoving(phase.isScrolling) }
        } else {
            content
        }
    }
}

private struct SheetTabs: View {
    let selected: SettingsGroup
    let edgePadding: CGFloat
    let onPick: (SettingsGroup) -> Void
    @Environment(\.theme) private var theme
    @Namespace private var indicator

    var body: some View {
        ScrollViewReader { proxy in
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 0) {
                    ForEach(SettingsGroup.allCases, id: \.self) { entry in
                        Button { onPick(entry) } label: {
                            Text(entry.title)
                                .font(.titleSmall)
                                .lineLimit(1)
                                .foregroundStyle(entry == selected ? theme.colors.primary : theme.colors.onSurfaceVariant)
                                .frame(height: Metrics.rowCompact)
                                .overlay(alignment: .bottom) {
                                    if entry == selected {
                                        UnevenRoundedRectangle(topLeadingRadius: 3, topTrailingRadius: 3)
                                            .fill(theme.colors.primary)
                                            .frame(height: 3)
                                            .matchedGeometryEffect(id: 0, in: indicator)
                                    }
                                }
                                .padding(.horizontal, 16)
                                .frame(minWidth: 90)
                                .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)
                        .accessibilityAddTraits(entry == selected ? .isSelected : [])
                        .id(entry)
                    }
                }
                .padding(.horizontal, edgePadding)
                .animation(.default, value: selected)
            }
            .onChange(of: selected, initial: true) { _, now in withAnimation { proxy.scrollTo(now, anchor: .center) } }
        }
    }
}

struct FlySettingsButton: View {
    @Environment(AppNavigationState.self) private var navigation
    @Environment(\.theme) private var theme

    var body: some View {
        Button { navigation.settingsOpen = true } label: {
            Text("⋯")
                .font(.titleLarge)
                .foregroundStyle(theme.aircast.outdoorForeground)
                .osdShadow()
                .frame(width: 48, height: 48)
                .contentShape(Rectangle())
                .macHover(Circle(), tint: theme.aircast.outdoorForeground)
        }
        .buttonStyle(.plain)
        .accessibilityLabel("Settings")
        .macTooltip("Settings")
    }
}
