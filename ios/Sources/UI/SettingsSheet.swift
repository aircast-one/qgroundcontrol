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
    @State private var setupOpen = false
    @State private var setupFromTab = false
    @State private var enteredForSetup = false
    @State private var page: String?
    @State private var query: String?
    @State private var returnQuery: String?
    @State private var openPage: String?
    @State private var heading = PageHeadingSlot()
    @State private var snackbars = SnackbarHostState()
    @State private var width: CGFloat = 0
    @FocusState private var searching: Bool

    init(requested: String?, onClose: @escaping () -> Void) {
        self.requested = requested
        self.onClose = onClose
        let opening = openingGroup(requested)
        _group = State(initialValue: opening)
        _page = State(initialValue: requested)
        _openPage = State(initialValue: shownOnOpen(requested, opening))
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

    private var notice: ChangeNotice {
        let host = snackbars
        return ChangeNotice { message, undo in
            Task { @MainActor in
                host.currentSnackbarData?.dismiss()
                let result = await host.showSnackbar(message, actionLabel: undo == nil ? nil : "Undo", duration: .Long)
                guard result == .ActionPerformed, let undo, let refused = await undo() else { return }
                await host.showSnackbar("Undo failed: \(refused)")
            }
        }
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
            if before == asked {
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
        .onDisappear { navigation.settingsShowing = nil }
    }

    @ViewBuilder
    private var header: some View {
        let stacked = width < STACKED_HEADER_WIDTH
        let drilled = sheetDrilled(openPage, setupOpen)
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
                        SheetTabs(selected: setupOpen ? nil : group, onPick: pickTab).frame(maxWidth: .infinity, alignment: .leading)
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
            .padding(.vertical, Space.s1)
            if stacked && query == nil && !drilled {
                SheetTabs(selected: setupOpen ? nil : group, onPick: pickTab)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, Space.s2)
            }
        }
    }

    @ViewBuilder
    private var content: some View {
        if query == nil && !setupOpen {
            SettingsPager(group: group, swipeable: openPage == nil, onSettled: pickTab) { shown in
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
    let group: SettingsGroup
    let swipeable: Bool
    let onSettled: (SettingsGroup) -> Void
    @ViewBuilder let content: (SettingsGroup) -> Content
    @State private var position: SettingsGroup?
    @State private var scrolling = false

    private func settle(_ now: SettingsGroup?) {
        guard !scrolling, let now, now != group else { return }
        onSettled(now)
    }

    var body: some View {
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
        .scrollTargetBehavior(.paging)
        .scrollIndicators(.hidden)
        .scrollPosition(id: $position)
        .environment(\.isScrollEnabled, swipeable)
        .onChange(of: group, initial: true) { _, now in if position != now { position = now } }
        .onChange(of: position) { _, now in settle(now) }
        .modifier(ScrollSettle { moving in
            scrolling = moving
            settle(position)
        })
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
    let selected: SettingsGroup?
    let onPick: (SettingsGroup) -> Void

    var body: some View {
        ScrollViewReader { proxy in
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: Space.s1) {
                    ForEach(SettingsGroup.allCases, id: \.self) { entry in
                        SheetTab(title: entry.title, selected: entry == selected) { onPick(entry) }.id(entry)
                    }
                }
            }
            .onChange(of: selected, initial: true) { _, now in
                guard let now else { return }
                withAnimation { proxy.scrollTo(now) }
            }
        }
    }
}

private struct SheetTab: View {
    let title: String
    let selected: Bool
    let onClick: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(spacing: 0) {
            Text(title)
                .font(.titleMedium)
                .fontWeight(selected ? .semibold : .regular)
                .foregroundStyle(selected ? theme.colors.onSurface : theme.colors.onSurfaceVariant)
            Capsule()
                .fill(selected ? theme.colors.primary : Color.clear)
                .frame(width: 20, height: 3)
                .padding(.top, 4)
        }
        .frame(minHeight: 48)
        .padding(.horizontal, Space.s2)
        .padding(.vertical, Space.s1)
        .contentShape(Rectangle())
        .onTapGesture(perform: onClick)
        .accessibilityElement(children: .combine)
        .accessibilityAddTraits(selected ? [.isButton, .isSelected] : .isButton)
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
