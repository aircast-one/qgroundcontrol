import SwiftUI

func sheetDrilled(_ openPage: String?, _ setupOpen: Bool) -> Bool { openPage != nil || setupOpen }

func openingGroup(_ requested: String?) -> SettingsGroup { requested.map { pageLook($0).group } ?? .Safety }

private let STACKED_HEADER_WIDTH: CGFloat = 600
private let SETTINGS_SHEET_LAYER: Double = 10
private let SETTINGS_PANEL_ALPHA = 0.9

private struct OpenPageKey: Hashable {
    let group: SettingsGroup
    let page: String?
    let searching: Bool
    let setupOpen: Bool
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

struct SettingsSheet: View {
    let requested: String?
    let onClose: () -> Void
    @Environment(AppNavigationState.self) private var navigation
    @Environment(\.theme) private var theme
    @FlyIsPortrait private var portrait
    @SettingsPages private var everyPage
    @State private var group: SettingsGroup
    @State private var pager: SettingsGroup?
    @State private var setupOpen = false
    @State private var setupFromTab = false
    @State private var enteredForSetup = false
    @State private var page: String?
    @State private var query: String?
    @State private var returnQuery: String?
    @State private var openPage: String?
    @State private var heading = PageHeadingSlot()
    @State private var snackbars: SnackbarHostState
    @State private var noticeScope: ViewScope
    @State private var notice: ChangeNotice
    @State private var width: CGFloat = 0
    @FocusState private var searching: Bool

    init(requested: String?, onClose: @escaping () -> Void) {
        self.requested = requested
        self.onClose = onClose
        let opening = openingGroup(requested)
        _group = State(initialValue: opening)
        _pager = State(initialValue: opening)
        _page = State(initialValue: requested)
        _openPage = State(initialValue: shownOnOpen(requested, opening))
        let host = SnackbarHostState()
        let scope = ViewScope()
        _snackbars = State(initialValue: host)
        _noticeScope = State(initialValue: scope)
        _notice = State(initialValue: settingsChangeNotice(host, scope))
    }

    private var initialPage: String? { page.flatMap { pageLook($0).group == group ? $0 : nil } }

    private func backToSearch() {
        query = returnQuery
        returnQuery = nil
        page = nil
    }

    private func closeSetup() {
        if enteredForSetup {
            onClose()
        } else {
            setupOpen = false
            if returnQuery != nil { backToSearch() }
        }
    }

    private func closePage() {
        if returnQuery != nil && openPage == initialPage {
            backToSearch()
        } else {
            openPage = nil
        }
    }

    private func openSetup(_ component: String?) {
        navigation.setupPage = component
        setupFromTab = component != nil
        setupOpen = true
        returnQuery = query.flatMap { $0.isBlank ? nil : $0 }
        query = nil
    }

    private func pickTab(_ entry: SettingsGroup) {
        group = entry
        page = nil
        setupOpen = false
        returnQuery = nil
    }

    var body: some View {
        VStack(spacing: 0) {
            header
            content.frame(maxHeight: .infinity)
        }
        .onGeometryChange(for: CGFloat.self) { $0.size.width } action: { width = $0 }
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
        .onChange(of: setupOpen) { _, open in if !open { enteredForSetup = false } }
        .onChange(of: navigation.aircraftRequested, initial: true) { before, asked in
            guard asked else { return }
            let openedForAircraft = before == asked
            if openedForAircraft {
                group = .General
                enteredForSetup = true
            }
            setupFromTab = setupFromTab || !(navigation.setupPage ?? "").isEmpty
            setupOpen = true
            query = nil
            navigation.aircraftRequested = false
        }
        .onChange(of: navigation.settingsPage, initial: true) { _, asked in
            guard let asked else { return }
            group = pageLook(asked).group
            page = asked
            setupOpen = false
            query = nil
            navigation.settingsPage = nil
        }
        .onChange(of: setupOpen ? nil : group, initial: true) { _, showing in navigation.settingsShowing = showing }
        .onChange(of: OpenPageKey(group: group, page: page, searching: query != nil, setupOpen: setupOpen)) { _, key in
            openPage = shownOnOpen(key.page, key.group)
        }
        .onDisappear {
            navigation.settingsShowing = nil
            noticeScope.cancel()
        }
    }

    @ViewBuilder
    private var header: some View {
        let stacked = width < STACKED_HEADER_WIDTH
        let drilled = sheetDrilled(openPage, setupOpen)
        let tabsShown = query == nil && !drilled
        let showTab: (SettingsGroup) -> Void = { entry in withAnimation { pager = entry } }
        VStack(spacing: 0) {
            HStack(spacing: Space.s2) {
                if let typed = query {
                    SearchPill(value: typed, onValueChange: { query = $0 }, placeholder: "Search settings")
                        .focused($searching)
                        .frame(maxWidth: .infinity)
                        .onAppear { searching = true }
                    Button("Cancel") { query = nil }.buttonStyle(.borderless)
                } else {
                    if stacked || drilled {
                        Text("Settings")
                            .font(.titleLarge)
                            .padding(.leading, Space.s2)
                            .frame(maxWidth: .infinity, alignment: .leading)
                    } else {
                        SheetTabs(selected: pager ?? group, edgePadding: 0, onPick: showTab).frame(maxWidth: .infinity, alignment: .leading)
                    }
                    Button { query = "" } label: { Image(.search).frame(width: 48, height: 48) }
                        .buttonStyle(.plain)
                        .accessibilityLabel("Search settings")
                }
                Button(action: onClose) { Image(.close).frame(width: 48, height: 48) }
                    .buttonStyle(.plain)
                    .accessibilityLabel("Close settings")
            }
            .padding(.horizontal, Space.s3)
            .padding(.top, Space.s1)
            .padding(.bottom, tabsShown && !stacked ? 0 : Space.s1)
            if stacked && tabsShown {
                SheetTabs(selected: pager ?? group, edgePadding: Space.s2, onPick: showTab)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            if tabsShown { Divider() }
        }
    }

    @ViewBuilder
    private var content: some View {
        if query == nil && !setupOpen {
            SettingsPager(pager: $pager, group: group, swipeable: openPage == nil, onSettled: pickTab) { shown in
                let current = shown == group
                SettingsScreen(
                    group: shown,
                    everyPage: everyPage,
                    open: current ? openPage : nil,
                    onOpen: { openPage = $0 },
                    onClose: closePage,
                    onOpenSetup: openSetup
                )
                .id(current ? page : nil)
            }
        } else {
            Group {
                if let typed = query {
                    SettingsSearch(query: typed, onOpenSetup: openSetup) { title in
                        group = pageLook(title).group
                        page = title
                        returnQuery = pageLook(title).inline ? nil : typed
                        query = nil
                        setupOpen = false
                    }
                } else {
                    VStack(spacing: 0) {
                        PageTopBar(title: heading.value?.title ?? AIRCRAFT_SETUP, backLabel: "Back", onBack: back)
                        SetupScreen()
                            .frame(maxHeight: .infinity)
                            .environment(\.LocalPageHeading, heading)
                    }
                }
            }
            .id(OpenPageKey(group: group, page: page, searching: query != nil, setupOpen: setupOpen))
        }
    }

    private func back() {
        if !setupFromTab, let pageBack = heading.value?.back {
            pageBack()
        } else {
            closeSetup()
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
        if #available(iOS 18, *) {
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
                                .frame(height: 48)
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
        }
        .buttonStyle(.plain)
        .accessibilityLabel("Settings")
    }
}
