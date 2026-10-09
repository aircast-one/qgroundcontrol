import Combine
import SwiftUI
import UIKit

private let SETTINGS_VIEW = "view.settings"
private let SEARCH_SETTLE_MS = 250

let UNITS_GROUP = "unitsSettings"
let FLY_VIEW_GROUP = "flyViewSettings"
let VIEWER_3D_GROUP = "viewer3DSettings"

let GROUPS_WITH_A_HEAD_EDITOR: Set<String> = [FLY_VIEW_GROUP]

let PAGES_WITHOUT_A_SCREEN: [String: String] = [
    "Firmware Upgrade": "QGC has no such settings page; the firmware screen reads these settings itself",
    "Flight Modes": "twelve comma-separated lists of hidden mode names, one per airframe. The mode " +
        "picker reads what they produce; the raw lists are worse than nothing",
]

let SECTIONS_DRAWN_BY_THE_HEAD: Set<String> = [MAVLINK_ACTIONS_GROUP]

let CONNECTIONS_PAGE = "Connections"

private let PAGE_GLANCES: [String: [String]] = [
    "Maps": ["settings.flightMapSettings.mapProvider", "settings.flightMapSettings.mapType"],
]

private func toDoubleOrNull(_ text: String) -> Double? { Double(text.trimmed) }

private func jsonNumber(_ value: JSON) -> Double? {
    if case .number(let number) = value { return number }
    return nil
}

private func distinct<T: Equatable>(_ items: [T]) -> [T] {
    items.enumerated().filter { at, item in !items[..<at].contains(item) }.map(\.element)
}

func glanceText(_ displays: [String]) -> String {
    displays.filter { !$0.isBlank }.map(sentenceCase).joined(separator: " · ")
}

private struct PageEntryRow: View {
    let page: SettingsPageEntry
    let linksJson: JSON?
    let onOpen: (String) -> Void
    @QgcDouble private var system: Double
    @QgcPath private var cameras: JSON?
    @QgcPath private var firstGlance: JSON?
    @QgcPath private var secondGlance: JSON?

    init(page: SettingsPageEntry, linksJson: JSON?, onOpen: @escaping (String) -> Void) {
        self.page = page
        self.linksJson = linksJson
        self.onOpen = onOpen
        let paths = PAGE_GLANCES[page.title] ?? []
        _system = QgcDouble(page.title == GENERAL_PAGE ? "settings.unitsSettings.unitSystem" : nil)
        _cameras = QgcPath(page.title == VIDEO_SOURCES_PAGE ? CAMERAS_VIEW : nil)
        _firstGlance = QgcPath(paths.first.map(settingControl))
        _secondGlance = QgcPath(paths.dropFirst().first.map(settingControl))
    }

    private var glance: String {
        if page.title == CONNECTIONS_PAGE { return activeLinksGlance(linksJson) }
        if page.title == "About" { return "Aircast \(qgcVersion())" }
        if page.title == GENERAL_PAGE { return system.isNaN ? "" : unitSystemLabel(Int(system)) }
        if page.title == VIDEO_SOURCES_PAGE { return camerasGlance(camerasReading(cameras)) }
        if PAGE_GLANCES[page.title] == nil { return "" }
        return glanceText([firstGlance, secondGlance].map { $0?["display"].string ?? "" })
    }

    var body: some View {
        SetupRow(title: pageTitle(page.title), status: glance, onClick: { onOpen(page.title) }, icon: pageLook(page.title).icon)
    }
}

private let PAGE_TITLES = [CONNECTIONS_PAGE: "Links", "ADSB Server": "ADS-B server"]

func pageTitle(_ title: String) -> String { PAGE_TITLES[title] ?? sentenceCase(title) }

func activeLinkCount(_ view: JSON?) -> Int {
    (view?["links"].array ?? []).filter { $0["connected"].bool }.count
}

func activeLinksGlance(_ view: JSON?) -> String {
    distinct(
        (view?["links"].array ?? [])
            .filter { $0.object != nil && $0["connected"].bool }
            .map { $0["summary"].string.ifBlank($0["name"].string) }
            .filter { !$0.isBlank }
    ).joined(separator: " \u{00b7} ")
}

func activeLinksText(_ count: Int) -> String { count > 0 ? "\(count) active" : "" }

enum SettingsGroup: String, CaseIterable {
    case Safety, Control, Camera, Transmission, General

    var title: String { rawValue }
}

struct PageLook: Equatable {
    let group: SettingsGroup
    let icon: Icon
    var inline: Bool = false
    var tabHome: Bool = false
}

private let PAGE_LOOK_ORDER: [(String, PageLook)] = [
    ("ADSB Server", PageLook(group: .Safety, icon: .navigation, inline: true)),
    ("Remote ID", PageLook(group: .Safety, icon: .shield)),
    ("Fly View", PageLook(group: .Control, icon: .flight, inline: true)),
    ("Flight Modes", PageLook(group: .Control, icon: .toggleOn)),
    ("Plan View", PageLook(group: .Control, icon: .route, inline: true)),
    ("Maps", PageLook(group: .Control, icon: .map)),
    ("3D Viewer", PageLook(group: .Control, icon: .explore)),
    ("Video", PageLook(group: .Camera, icon: .videocam, inline: true)),
    ("Connections", PageLook(group: .Transmission, icon: .link)),
    (VIDEO_SOURCES_PAGE, PageLook(group: .Transmission, icon: .videocam)),
    ("MAVLink", PageLook(group: .Transmission, icon: .swapHoriz)),
    ("Packet Radio", PageLook(group: .Transmission, icon: .wifi)),
    ("RTK GPS", PageLook(group: .Transmission, icon: .satelliteAlt)),
    ("NTRIP / RTK", PageLook(group: .Transmission, icon: .satelliteAlt)),
    ("General", PageLook(group: .General, icon: .tune, inline: true, tabHome: true)),
    ("PX4 Log Transfer", PageLook(group: .General, icon: .download)),
    ("Firmware Upgrade", PageLook(group: .General, icon: .developerBoard)),
    ("App Logging", PageLook(group: .General, icon: .description)),
    ("Console", PageLook(group: .General, icon: .terminal)),
    ("About", PageLook(group: .General, icon: .help)),
]

let PAGE_LOOKS: [String: PageLook] = Dictionary(PAGE_LOOK_ORDER, uniquingKeysWith: { first, _ in first })

let BLOCK_HOMES: [(page: String, block: String, group: SettingsGroup)] = [("Fly View", "Guided Commands", .Safety)]

func pageLook(_ title: String) -> PageLook { PAGE_LOOKS[title] ?? PageLook(group: .General, icon: .settings) }

func blockGroup(_ page: String, _ block: SettingsBlock) -> SettingsGroup {
    BLOCK_HOMES.first { $0.page == page && $0.block == block.title }?.group ?? pageLook(page).group
}

private func lookOrder(_ title: String) -> Int { PAGE_LOOK_ORDER.firstIndex { $0.0 == title } ?? Int.max }

func tabPages(_ group: SettingsGroup, _ pages: [SettingsPageEntry]) -> [SettingsPageEntry] {
    let lends = BLOCK_HOMES.filter { $0.group == group }.map(\.page)
    let own = pages.enumerated()
        .filter { pageLook($0.element.title).group == group }
        .sorted { (lookOrder($0.element.title), $0.offset) < (lookOrder($1.element.title), $1.offset) }
        .map(\.element)
    return pages.filter { lends.contains($0.title) && pageLook($0.title).group != group } + own
}

func sectionsIn(_ group: SettingsGroup, _ page: String, _ sections: [SettingsSectionRows]) -> [SettingsSectionRows] {
    sections
        .map { SettingsSectionRows(title: $0.title, group: $0.group, note: $0.note, blocks: $0.blocks.filter { blockGroup(page, $0) == group }) }
        .filter { !$0.blocks.isEmpty }
}

struct HelpLink: Equatable, Hashable {
    let name: String
    let url: String
    let host: String
}

struct SettingsPageEntry: Equatable {
    let title: String
    let showsLinks: Bool
    let showsVideoSources: Bool
    let sectionCount: Int
    var showsAbout: Bool = false
    var showsConsole: Bool = false
    var showsNtrip: Bool = false
    var showsPx4Logs: Bool = false
    var showsPacketRadio: Bool = false
    var helpLinks: [HelpLink] = []
    var keywords: String = ""
}

struct SettingsBlock: Equatable {
    let title: String
    let facts: [Fact]
}

struct SettingsSectionRows: Equatable {
    let title: String
    let group: String
    let note: String
    let blocks: [SettingsBlock]
}

func settingsPagePath(_ title: String) -> String { "\(SETTINGS_VIEW)(\(title))" }

@MainActor private final class SettingsPagesLoader: ObservableObject {
    @Published var pages: [SettingsPageEntry] = []

    init() {
        offMain { [weak self] in
            let read = settingsPages(Qgc.get(SETTINGS_VIEW))
            onMain { self?.pages = read }
        }
    }
}

@propertyWrapper
struct SettingsPages: DynamicProperty {
    @StateObject private var loader = SettingsPagesLoader()

    init() {}

    var wrappedValue: [SettingsPageEntry] { loader.pages }
}

func editOnDesktop(_ fact: Fact) -> Bool { !controlIsUnderstood(fact.controlKind) }

func inertNote(_ fact: Fact) -> String {
    !fact.enabled ? fact.disabledReason.ifBlank("Has no effect yet") : "Read-only"
}

func settingsPages(_ view: JSON?) -> [SettingsPageEntry] {
    (view?["pages"].arrayOrNil ?? []).filter { $0.object != nil }.map { page in
        SettingsPageEntry(
            title: page["title"].string,
            showsLinks: page["showsLinks"].bool,
            showsVideoSources: page["showsVideoSources"].bool,
            sectionCount: page["sections"].array.count,
            showsAbout: page["showsAbout"].bool,
            showsConsole: page["showsConsole"].bool,
            showsNtrip: page["showsNtrip"].bool,
            showsPx4Logs: page["showsPx4Logs"].bool,
            showsPacketRadio: page["showsPacketRadio"].bool,
            helpLinks: page["helpLinks"].array.filter { $0.object != nil }.map {
                HelpLink(name: $0["name"].string, url: $0["url"].string, host: $0["host"].string)
            },
            keywords: page["keywords"].string
        )
    }.filter {
        !$0.title.isBlank && PAGES_WITHOUT_A_SCREEN[$0.title] == nil
            && ($0.sectionCount > 0 || $0.showsLinks || $0.showsAbout || $0.showsConsole || $0.showsPx4Logs || $0.showsVideoSources)
    }
}

func settingsSections(_ page: JSON?) -> [SettingsSectionRows] {
    (page?["sections"].arrayOrNil ?? []).filter { $0.object != nil }.map { section in
        SettingsSectionRows(
            title: section["title"].string,
            group: section["group"].string,
            note: section["note"].string,
            blocks: section["subsections"].array.filter { $0.object != nil }.map { block in
                SettingsBlock(
                    title: block["title"].string,
                    facts: block["controls"].array.filter { $0.object != nil }.compactMap(factFromControl).map(paletteNamed).map(pilotWorded)
                )
            }.filter { !$0.facts.isEmpty }
        )
    }
    .map {
        SECTIONS_DRAWN_BY_THE_HEAD.contains($0.group)
            ? SettingsSectionRows(title: $0.title, group: $0.group, note: $0.note, blocks: [SettingsBlock(title: "", facts: [])])
            : $0
    }
    .filter { !$0.blocks.isEmpty }
}

func blockHeading(_ pageTitle: String, _ section: SettingsSectionRows, _ block: SettingsBlock) -> String {
    block.title.ifBlank(section.title != pageTitle ? section.title : "")
}

func shownBreadcrumb(_ title: String) -> String {
    title.components(separatedBy: " \u{203a} ").map(sentenceCase).joined(separator: " \u{203a} ")
}

func pageMatches(_ page: SettingsPageEntry, _ needle: String) -> Bool {
    let wanted = needle.trimmed.lowercased()
    return !wanted.isEmpty && (page.title.lowercased().contains(wanted) || page.keywords.contains(wanted))
}

func matchesIn(_ pageTitle: String, _ sections: [SettingsSectionRows], _ needle: String) -> [SettingsSectionRows] {
    let wanted = needle.trimmed.lowercased()
    if wanted.isBlank { return [] }
    return sections.filter { $0.group != UNITS_GROUP }.compactMap { section in
        let hits = section.blocks.flatMap(\.facts).filter {
            $0.title.lowercased().contains(wanted) || $0.name.lowercased().contains(wanted) || $0.keywords.contains(wanted)
        }
        return hits.isEmpty
            ? nil
            : SettingsSectionRows(title: "\(pageTitle) \u{203a} \(section.title)", group: section.group, note: "", blocks: [SettingsBlock(title: "", facts: hits)])
    }
}

let AIRCRAFT_SETUP = "Aircraft setup"

let TAB_SETUP_TITLES = [SAFETY_SETUP_PAGE: "More failsafe settings"]

func tabSetupPages(_ group: SettingsGroup) -> [String] {
    switch group {
    case .Safety: [SAFETY_SETUP_PAGE]
    case .Control: [FLIGHT_MODES_PAGE]
    default: []
    }
}

func tabSetupComponents(_ group: SettingsGroup, _ components: [SetupComponent]) -> [SetupComponent] {
    tabSetupPages(group).compactMap { name in components.first { $0.name == name } }
}

func setupSearchHits(_ components: [SetupComponent], _ query: String) -> [SetupComponent] {
    query.isBlank ? [] : components.filter { setupMatches($0.name, query) }
}

private struct TabSetupRows: View {
    let group: SettingsGroup
    let onOpenSetup: (String?) -> Void
    @QgcPath(SETUP) private var setupJson: JSON?

    var body: some View {
        ForEach(tabSetupComponents(group, setupComponents(setupJson)), id: \.name) { component in
            SetupRow(
                title: TAB_SETUP_TITLES[component.name] ?? sentenceCase(component.name),
                status: "",
                onClick: { onOpenSetup(component.name) },
                icon: setupIcon(component.known, className: component.className)
            )
        }
    }
}

private struct AircraftSetupRow: View {
    let onOpenSetup: (String?) -> Void

    var body: some View {
        SetupRow(title: AIRCRAFT_SETUP, status: "", onClick: { onOpenSetup(nil) }, icon: .build)
    }
}

struct SettingsScreen: View {
    let group: SettingsGroup
    let everyPage: [SettingsPageEntry]
    let open: String?
    let onOpen: (String) -> Void
    let onClose: () -> Void
    var onOpenSetup: (String?) -> Void = { _ in }
    @State private var heading = PageHeadingSlot()

    var body: some View {
        let current = everyPage.first { $0.title == open }
        Group {
            if let current {
                VStack(spacing: 0) {
                    PageTopBar(title: heading.value?.title ?? pageTitle(current.title), backLabel: "Back") {
                        if let back = heading.value?.back { back() } else { onClose() }
                    }
                    SettingsPageBody(page: current)
                        .id(current.title)
                        .frame(maxHeight: .infinity, alignment: .top)
                }
            } else {
                SettingsTab(group: group, everyPage: everyPage, onOpenSetup: onOpenSetup, onOpen: onOpen)
                    .id(group)
            }
        }
        .frame(maxWidth: DETAIL_PANE_MAX_WIDTH, maxHeight: .infinity, alignment: .top)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .environment(\.LocalPageHeading, heading)
        .environment(\.LocalSettingsList, true)
    }
}

let LIST_DETAIL_MIN_WIDTH: CGFloat = 840
let LIST_PANE_WIDTH: CGFloat = 380
let DETAIL_PANE_MAX_WIDTH: CGFloat = 720

private struct SettingsTab: View {
    let group: SettingsGroup
    let everyPage: [SettingsPageEntry]
    let onOpenSetup: (String?) -> Void
    let onOpen: (String) -> Void
    @QgcPath("view.links") private var linksJson: JSON?
    @State private var advancedOpen = false

    private func drawnInline(_ page: SettingsPageEntry) -> Bool {
        pageLook(page.title).inline || pageLook(page.title).group != group
    }

    @ViewBuilder private func pageEntry(_ page: SettingsPageEntry) -> some View {
        if drawnInline(page) {
            InlinePage(page: page, group: group)
        } else {
            PageEntryRow(page: page, linksJson: linksJson, onOpen: onOpen)
        }
    }

    var body: some View {
        let pages = tabPages(group, everyPage)
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                if group == .General { AircraftSetupRow(onOpenSetup: onOpenSetup) }
                PilotSettings(group: group)
                if group == .Safety { SensorChecks(onCalibrate: { onOpenSetup(SENSORS) }) }
                if group == .Safety {
                    let folded = pages.filter(drawnInline)
                    ForEach(pages.filter { !drawnInline($0) }, id: \.title) { pageEntry($0) }
                    TabSetupRows(group: group, onOpenSetup: onOpenSetup)
                    if !folded.isEmpty {
                        AdvancedToggle(open: advancedOpen, label: ADVANCED_SAFETY) { advancedOpen.toggle() }
                    }
                    if advancedOpen { ForEach(folded, id: \.title) { pageEntry($0) } }
                } else {
                    ForEach(pages, id: \.title) { pageEntry($0) }
                    TabSetupRows(group: group, onOpenSetup: onOpenSetup)
                }
            }
        }
    }
}

private struct InlinePage: View {
    let page: SettingsPageEntry
    let group: SettingsGroup
    @State private var sections: [SettingsSectionRows]? = nil
    @State private var reloads = 0

    private struct Load: Equatable {
        let title: String
        let reloads: Int
    }

    var body: some View {
        let shown = sections.map { sectionsIn(group, page.title, $0) } ?? []
        let home = pageLook(page.title).group == group
        VStack(alignment: .leading, spacing: 0) {
            Color.clear.frame(height: 0).task(id: Load(title: page.title, reloads: reloads)) {
                let title = page.title
                sections = await offMain { settingsSections(Qgc.get(settingsPagePath(title))) }
            }
            if !shown.isEmpty {
                if !(home && pageLook(page.title).tabHome) {
                    PageHeader(icon: pageLook(page.title).icon, title: home ? pageTitle(page.title) : borrowedTitle(shown))
                }
                SettingsControls(page: page, sections: shown, home: home, blockHeadings: home) { reloads += 1 }
                if home && page.title == GENERAL_PAGE { ResetAllSettingsRow() }
            }
        }
    }
}

func borrowedTitle(_ sections: [SettingsSectionRows]) -> String {
    distinct(sections.flatMap(\.blocks).map { sentenceCase($0.title) }.filter { !$0.isBlank }).joined(separator: " \u{00b7} ")
}

private struct PageHeader: View {
    let icon: Icon
    let title: String
    @Environment(\.theme) private var theme

    var body: some View {
        HStack(spacing: 12) {
            Image(icon).font(.system(size: 20)).foregroundStyle(theme.colors.primary).frame(width: 24, height: 24)
            Text(title).font(.titleMedium)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(EdgeInsets(top: 28, leading: 16, bottom: 4, trailing: 16))
    }
}

@MainActor private final class PagesWatch: ObservableObject {
    private var watchers: [PathWatcher] = []
    private var forwarding: [AnyCancellable] = []

    var served: [JSON?] { watchers.map(\.value) }

    func follow(_ paths: [String]) {
        guard paths != watchers.compactMap(\.path) else { return }
        objectWillChange.send()
        watchers = paths.map { PathWatcher($0) }
        forwarding = watchers.map { $0.objectWillChange.sink { [weak self] _ in self?.objectWillChange.send() } }
    }
}

struct SettingsSearch: View {
    let query: String
    var onOpenSetup: (String?) -> Void = { _ in }
    let onOpen: (String) -> Void
    @QgcPath(SETUP) private var setupJson: JSON?
    @State private var pages: [SettingsPageEntry] = []
    @State private var hits: [SettingsSectionRows] = []
    @StateObject private var watch = PagesWatch()

    private struct Search: Equatable {
        let query: String
        let pages: [SettingsPageEntry]
        let served: [JSON?]
    }

    var body: some View {
        let searching = !query.isBlank
        let setupHits = setupSearchHits(setupComponents(setupJson), query)
        let pageHits = pages.filter { pageMatches($0, query) }
        let pagePaths = searching ? pages.map { settingsPagePath($0.title) } : []
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 0) {
                if searching {
                    if hits.isEmpty && pageHits.isEmpty && setupHits.isEmpty {
                        FootNote(text: "No settings match \u{201c}\(query.trimmed)\u{201d}.")
                    } else {
                        if !setupHits.isEmpty { SectionHeader(text: AIRCRAFT_SETUP) }
                        ForEach(setupHits, id: \.name) { component in
                            SetupRow(title: sentenceCase(component.name), onClick: { onOpenSetup(component.name) }, icon: setupIcon(component.known, className: component.className))
                        }
                        if !pageHits.isEmpty { SectionHeader(text: "Pages") }
                        ForEach(pageHits, id: \.title) { entry in
                            SetupRow(title: pageTitle(entry.title), onClick: { onOpen(entry.title) }, icon: pageLook(entry.title).icon)
                        }
                        ForEach(hits, id: \.title) { section in
                            SectionHeader(text: shownBreadcrumb(section.title))
                            ForEach(section.blocks.flatMap(\.facts)) { fact in
                                FactRow(fact: fact)
                            }
                        }
                    }
                }
            }
            .frame(maxWidth: DETAIL_PANE_MAX_WIDTH)
            .frame(maxWidth: .infinity)
        }
        .task {
            pages = await offMain { settingsPages(Qgc.get(SETTINGS_VIEW)) }
        }
        .onChange(of: pagePaths, initial: true) { watch.follow(pagePaths) }
        .task(id: Search(query: query, pages: pages, served: watch.served)) {
            guard searching else {
                hits = []
                return
            }
            try? await Task.sleep(for: .milliseconds(SEARCH_SETTLE_MS))
            if Task.isCancelled { return }
            let pages = pages, served = watch.served, query = query
            hits = await offMain {
                pages.enumerated().flatMap { at, page in
                    matchesIn(page.title, settingsSections((served.indices.contains(at) ? served[at] : nil) ?? Qgc.get(settingsPagePath(page.title))), query)
                }
            }
        }
    }
}

private struct SettingsPageBody: View {
    let page: SettingsPageEntry
    @State private var sections: [SettingsSectionRows] = []
    @State private var loaded = false
    @State private var reloads = 0

    var body: some View {
        content.task(id: reloads) {
            let title = page.title
            sections = await offMain { settingsSections(Qgc.get(settingsPagePath(title))) }
            loaded = true
        }
    }

    @ViewBuilder private var content: some View {
        if page.showsLinks {
            LinksScreen(footer: { SettingsControls(page: page, sections: sections) { reloads += 1 } })
        } else if page.showsAbout {
            AboutPage(links: page.helpLinks)
        } else if page.showsConsole {
            AppLogPage()
        } else if page.showsPx4Logs {
            Px4LogTransferPage()
        } else if page.showsVideoSources {
            CamerasEditor()
        } else if !loaded {
            Text("Reading settings.").padding(16).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        } else if sections.isEmpty {
            Text("No settings exposed here.").padding(16).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        } else {
            ScrollView {
                VStack(alignment: .leading, spacing: 0) {
                    SettingsControls(page: page, sections: sections) { reloads += 1 }
                    if page.title == GENERAL_PAGE { ResetAllSettingsRow() }
                }
            }
        }
    }
}

private struct SettingsControls: View {
    let page: SettingsPageEntry
    let sections: [SettingsSectionRows]
    var home: Bool = true
    var blockHeadings: Bool = true
    let onWrite: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if home && page.showsNtrip { NtripStatusSection(onWrite: onWrite) }
            ForEach(Array(sections.enumerated()), id: \.offset) { _, section in
                sectionView(section)
            }
        }
    }

    @ViewBuilder private func sectionView(_ section: SettingsSectionRows) -> some View {
        if section.group == UNITS_GROUP {
            SectionHeader(text: section.title)
            UnitsSection()
        } else {
            ForEach(Array(section.blocks.enumerated()), id: \.offset) { _, block in
                blockView(section, block)
            }
            if home { sectionExtras(section) }
        }
    }

    @ViewBuilder private func sectionExtras(_ section: SettingsSectionRows) -> some View {
        if !section.note.isBlank && !GROUPS_WITH_A_HEAD_EDITOR.contains(section.group) { FootNote(text: section.note) }
        if section.group == FLY_VIEW_GROUP { RcControlsEditor() }
        if section.group == VIEWER_3D_GROUP { OsmFilePicker(onWrite: onWrite) }
        if section.group == OFFLINE_MAPS_GROUP { OfflineMapsSection() }
        if section.group == MAVLINK_GROUP { LinkStatusSection() }
        if section.group == MAVLINK_ACTIONS_GROUP { MavlinkActionsSection(onWrite: onWrite) }
    }

    @ViewBuilder private func blockView(_ section: SettingsSectionRows, _ block: SettingsBlock) -> some View {
        if block.title == ADVANCED_BLOCK {
            AdvancedBlock(key: "\(section.group)#\(block.title)") { FactRuns(facts: block.facts, onWrite: onWrite) }
        } else {
            let heading = blockHeading(page.title, section, block)
            if blockHeadings && !heading.isBlank { SectionHeader(text: sentenceCase(heading)) }
            FactRuns(facts: block.facts, onWrite: onWrite)
            if page.showsNtrip && block.title == NTRIP_MOUNTPOINT_BLOCK { NtripMountpointBrowser(onWrite: onWrite) }
            if section.group == REMOTE_ID_GROUP && block.title == GCS_LOCATION_BLOCK { GcsPositionStatus() }
            if section.group == MAVLINK_GROUP && block.title == SIGNING_AFTER_BLOCK { SigningKeysSection() }
        }
    }
}

let ADVANCED_BLOCK = "Advanced"

private struct AdvancedBlock<Content: View>: View {
    let key: String
    @ViewBuilder let content: () -> Content
    @Environment(\.theme) private var theme
    @State private var open = false

    var body: some View {
        Button { open.toggle() } label: {
            HStack {
                Text(ADVANCED_BLOCK).font(.labelLarge).foregroundStyle(theme.colors.onSurfaceVariant).frame(maxWidth: .infinity, alignment: .leading)
                Image(.arrowDropDown).foregroundStyle(theme.colors.onSurfaceVariant).rotationEffect(.degrees(open ? 180 : 0))
            }
            .padding(EdgeInsets(top: 20, leading: 16, bottom: 8, trailing: 16))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityHint(open ? "Hide advanced settings" : "Show advanced settings")
        if open { content() }
    }
}

struct FactRuns: View {
    let facts: [Fact]
    var onWrite: () -> Void = {}
    @Environment(\.theme) private var theme

    var body: some View {
        let shared = sharedRebootNote(facts)
        let inert = blockInertNote(facts)
        if let shared {
            Text(shared).font(.bodySmall).foregroundStyle(theme.aircast.warning).padding(.horizontal, 16).padding(.vertical, 4)
        }
        ForEach(facts) { fact in
            FactRow(fact: fact, onWrite: onWrite)
        }
        .environment(\.LocalBlockRebootNote, shared)
        .environment(\.LocalRunInertNote, inert)
        if let inert {
            Text(inert).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant).padding(.horizontal, 16).padding(.vertical, 4)
        }
    }
}

extension EnvironmentValues {
    @Entry var LocalBlockRebootNote: String? = nil
    @Entry var LocalRunInertNote: String? = nil
    @Entry var LocalSettingsList = false
}

let SUBTITLE_SEPARATOR = " · "

struct ShownSubtitle: Equatable {
    let text: String
    let hasHelp: Bool
}

func shownSubtitle(_ subtitle: String, _ detail: String, _ helpBehind: Bool, _ helpOpen: Bool) -> ShownSubtitle {
    let hasHelp = helpBehind && !detail.isBlank && (subtitle == detail || subtitle.hasPrefix(detail + SUBTITLE_SEPARATOR))
    let text = hasHelp && !helpOpen ? subtitle.removingPrefix(detail).removingPrefix(SUBTITLE_SEPARATOR) : subtitle
    return ShownSubtitle(text: text, hasHelp: hasHelp)
}

private struct FactTitle: View {
    let title: String
    let color: Color?
    let helpOpen: Bool?
    let onHelp: () -> Void
    var maxLines: Int? = nil
    @Environment(\.theme) private var theme
    @Environment(\.LocalSettingsList) private var settingsList

    var body: some View {
        HStack(spacing: 0) {
            Text(title)
                .font(.bodyLarge)
                .fontWeight(settingsList ? .medium : nil)
                .foregroundStyle(color ?? theme.colors.onSurface)
                .lineLimit(maxLines)
                .fixedSize(horizontal: false, vertical: true)
            if let helpOpen {
                Button(action: onHelp) {
                    Image(.help)
                        .font(.system(size: 14))
                        .foregroundStyle(helpOpen ? theme.colors.primary : theme.colors.onSurfaceVariant)
                        .frame(width: 40, height: 40)
                }
                .buttonStyle(.plain)
                .accessibilityLabel(helpOpen ? "Hide help" : "Help")
            }
        }
    }
}

func blockInertNote(_ facts: [Fact]) -> String? {
    let off = facts.filter { !$0.enabled }
    guard off.count > 1 else { return nil }
    let notes = distinct(off.map(inertNote))
    return notes.count == 1 ? notes[0] : nil
}

func sharedRebootNote(_ facts: [Fact]) -> String? {
    let notes = facts.compactMap(factRebootNote)
    guard notes.count > 1 else { return nil }
    let kinds = distinct(notes)
    return kinds.count == 1 ? kinds[0] : nil
}

let NTRIP_MOUNTPOINT_BLOCK = "Mountpoint"

func isSecret(_ fact: Fact) -> Bool { fact.name.lowercased().hasSuffix("password") }

func enumLabel(_ fact: Fact) -> String {
    fact.enumStrings.indices.contains(fact.enumIndex) ? fact.enumStrings[fact.enumIndex] : fact.valueString
}

func shownEnumLabel(_ fact: Fact) -> String {
    fact.enumStrings.indices.contains(fact.enumIndex) ? sentenceCase(fact.enumStrings[fact.enumIndex]) : fact.valueString
}

func factSubtitle(_ fact: Fact) -> String {
    !fact.enumStrings.isEmpty || !fact.bitmaskStrings.isEmpty || fact.isBool ? "" : fact.units
}

struct FieldSlider: View {
    let value: Double?
    let slider: FactSlider
    let enabled: Bool
    let onWrite: (Double) -> Void
    @Environment(\.theme) private var theme
    @State private var shown: Double? = nil

    private var range: ClosedRange<Double> { min(slider.from, slider.to)...max(slider.from, slider.to) }

    var body: some View {
        let held = min(max(value ?? slider.from, range.lowerBound), range.upperBound)
        VStack(spacing: 0) {
            Slider(value: Binding(get: { shown ?? held }, set: { shown = $0 }), in: range) { editing in
                if !editing { onWrite(shown ?? held) }
            }
            .disabled(!enabled)
            HStack {
                Text(sliderValue(slider.from, slider.decimals, "")).font(.labelSmall).foregroundStyle(theme.colors.onSurfaceVariant)
                Spacer()
                Text(sliderValue(slider.to, slider.decimals, "")).font(.labelSmall).foregroundStyle(theme.colors.onSurfaceVariant)
            }
        }
        .onChange(of: value) { shown = nil }
        .onChange(of: slider) { shown = nil }
    }
}

struct FactRow: View {
    let fact: Fact
    let title: String
    let subtitle: String
    let titleColor: Color?
    let fieldModifier: EdgeInsets
    let warning: String?
    let onRejected: () -> Void
    let onWrite: () -> Void
    @Environment(\.theme) private var theme
    @Environment(\.LocalSettingsList) private var settingsList
    @Environment(\.LocalRunInertNote) private var runInert
    @State private var refusal: String? = nil
    @State private var helpOpen = false
    @State private var editing = false

    init(
        fact: Fact,
        title: String? = nil,
        subtitle: String? = nil,
        titleColor: Color? = nil,
        fieldModifier: EdgeInsets = EdgeInsets(top: 8, leading: 16, bottom: 8, trailing: 16),
        warning: String? = nil,
        onRejected: @escaping () -> Void = {},
        onWrite: @escaping () -> Void = {}
    ) {
        self.fact = fact
        self.title = title ?? sentenceCase(fact.heading)
        self.subtitle = subtitle ?? [fact.detail, factSubtitle(fact)].filter { !$0.isBlank }.joined(separator: " · ")
        self.titleColor = titleColor
        self.fieldModifier = fieldModifier
        self.warning = warning
        self.onRejected = onRejected
        self.onWrite = onWrite
    }

    private var shown: ShownSubtitle { shownSubtitle(subtitle, fact.detail, settingsList, helpOpen) }

    private var helpToggle: Bool? { shown.hasHelp ? helpOpen : nil }

    private var segmented: Bool {
        !editOnDesktop(fact) && !fact.isBitmask && showsAsSegments(fact.isEnum, fact.valueIsOffTheEnumList, fact.acceptsWrite, fact.enumStrings)
    }

    private var asField: Bool { !segmented && showsAsField(fact) }

    private var rowToggles: Bool { !segmented && fact.isBool && fact.acceptsWrite && !editOnDesktop(fact) }

    private func write(_ block: @escaping @Sendable () -> Bool) {
        let onWrite = onWrite
        Task { @MainActor in
            let accepted = await offMain(block)
            refusal = writeRefusal(accepted)
            if accepted { onWrite() }
        }
    }

    private func toggleRow() {
        let path = fact.path, checked = !fact.boolValue, inverted = fact.inverted
        write { Qgc.set(path, checked != inverted) }
    }

    var body: some View {
        Group {
            if asField && settingsList && opensAsValue(fact) {
                valueRow
            } else if asField {
                fieldColumn
            } else if segmented {
                ViewThatFits(in: .horizontal) {
                    rows(true).frame(minWidth: SEGMENTS_BESIDE_MIN_WIDTH)
                    rows(false)
                }
            } else {
                rows(false)
            }
        }
        .onChange(of: fact.path) {
            refusal = nil
            helpOpen = false
            editing = false
        }
    }

    private var valueTitle: some View {
        let note = shown.text.components(separatedBy: SUBTITLE_SEPARATOR)
            .filter { !$0.isBlank && $0 != fact.units && $0 != fact.detail }
            .joined(separator: SUBTITLE_SEPARATOR)
        return VStack(alignment: .leading, spacing: 2) {
            FactTitle(title: title, color: titleColor, helpOpen: nil, onHelp: {})
            if !note.isBlank { Text(note).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant) }
            if let warning { Text(warning).font(.bodySmall).foregroundStyle(theme.aircast.warning) }
        }
    }

    private var valueRow: some View {
        Group {
            if let slider = inlineSlider(fact) {
                SliderValueRow(fact: fact, label: title, slider: slider, title: { valueTitle }, onOpen: { editing = true }, onWrite: onWrite)
            } else {
                Button { editing = true } label: {
                    HStack(spacing: 12) {
                        valueTitle.frame(maxWidth: .infinity, alignment: .leading)
                        Text(valueText(fact)).font(.bodyLarge).foregroundStyle(theme.colors.onSurfaceVariant)
                        Image(.chevronRight).foregroundStyle(theme.colors.onSurfaceVariant)
                    }
                    .padding(.horizontal, 16)
                    .padding(.vertical, 10)
                    .frame(maxWidth: .infinity, minHeight: 56)
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
            }
        }
        .background {
            if editing {
                ValueDetailsSheet(fact: fact, title: title, onWrite: onWrite, onDismiss: { editing = false })
            }
        }
    }

    private var fieldColumn: some View {
        let inert = !fact.enabled && inertNote(fact) != runInert ? [inertNote(fact)] : []
        let note = (shown.text.components(separatedBy: SUBTITLE_SEPARATOR) + inert)
            .filter { !$0.isBlank && $0 != fact.units }
            .joined(separator: " · ")
        return VStack(alignment: .leading, spacing: 0) {
            if valueOnTheRight(fact) {
                HStack(spacing: 16) {
                    VStack(alignment: .leading, spacing: 2) {
                        FactTitle(title: title, color: titleColor, helpOpen: helpToggle, onHelp: { helpOpen.toggle() })
                        if !note.isBlank { Text(note).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant) }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    if fact.isEnum && !fact.valueIsOffTheEnumList {
                        EnumField(fact: fact, write: write).frame(width: CHOICE_VALUE_WIDTH)
                    } else {
                        FactTextField(fact: fact, onWrite: onWrite, onRejected: onRejected, onTheRight: true)
                    }
                }
            } else {
                FactTitle(title: title, color: titleColor, helpOpen: helpToggle, onHelp: { helpOpen.toggle() }).padding(.bottom, 8)
                if fact.isBitmask {
                    BitmaskPicker(fact: fact, write: write)
                } else {
                    FactTextField(fact: fact, onWrite: onWrite, onRejected: onRejected)
                }
            }
            if let slider = fact.slider, !fact.isEnum, !fact.isBitmask {
                FieldSlider(value: jsonNumber(fact.value) ?? toDoubleOrNull(fact.valueString), slider: slider, enabled: fact.acceptsWrite) { value in
                    let path = fact.path
                    write { Qgc.set(path, value) }
                }
            }
            if !note.isBlank && !valueOnTheRight(fact) {
                Text(note).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant).padding(.leading, 16).padding(.top, 4)
            }
            if let refusal {
                Text(refusal).font(.bodySmall).foregroundStyle(theme.colors.error).padding(.leading, 16).padding(.top, 4)
            }
        }
        .padding(fieldModifier)
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private func rows(_ beside: Bool) -> some View {
        let rowNote = shown.text.components(separatedBy: SUBTITLE_SEPARATOR)
            .filter { !$0.isBlank && $0 != fact.units }
            .joined(separator: " · ")
        return VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 16) {
                VStack(alignment: .leading, spacing: 2) {
                    FactTitle(title: title, color: titleColor, helpOpen: helpToggle, onHelp: { helpOpen.toggle() }, maxLines: 3)
                    if !rowNote.isBlank && rowNote.lowercased() != title.lowercased() {
                        Text(rowNote).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant).lineLimit(2)
                    }
                    if !fact.acceptsWrite && !editOnDesktop(fact) {
                        Text(inertNote(fact)).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
                    }
                }
                .frame(minWidth: 0, idealWidth: beside ? 0 : nil, maxWidth: .infinity, alignment: .leading)
                .contentShape(Rectangle())
                .onTapGesture { if rowToggles { toggleRow() } }
                if beside { Segments(fact: fact, write: write) }
                if !segmented { CappedWidth(limit: TRAILING_MAX_WIDTH) { trailing } }
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 10)
            .frame(maxWidth: .infinity, minHeight: 64)
            if segmented && !beside {
                Segments(fact: fact, write: write).padding(EdgeInsets(top: 0, leading: 16, bottom: 10, trailing: 16))
            }
            if let refusal {
                Text(refusal).font(.bodySmall).foregroundStyle(theme.colors.error).padding(.leading, 16).padding(.bottom, 8)
            }
        }
    }

    @ViewBuilder private var trailing: some View {
        if editOnDesktop(fact) {
            VStack(alignment: .trailing, spacing: 2) {
                Text(shownEnumLabel(fact)).font(.bodyMedium).lineLimit(2).multilineTextAlignment(.trailing)
                Text("Edit on desktop").font(.labelSmall).foregroundStyle(theme.colors.onSurfaceVariant)
            }
        } else if !fact.acceptsWrite {
            if fact.isBool {
                Toggle(title, isOn: .constant(fact.boolValue)).labelsHidden().disabled(true)
            } else {
                Text([shownEnumLabel(fact), fact.isEnum ? "" : fact.units].filter { !$0.isBlank }.joined(separator: " "))
                    .font(.bodyMedium)
                    .lineLimit(2)
                    .multilineTextAlignment(.trailing)
            }
        } else if fact.isBool {
            Toggle(title, isOn: Binding(get: { fact.boolValue }, set: { _ in toggleRow() })).labelsHidden()
        } else {
            BitmaskPicker(fact: fact, write: write)
        }
    }
}

private struct Segments: View {
    let fact: Fact
    let write: (@escaping @Sendable () -> Bool) -> Void

    private func boolOption(_ index: Int) -> Bool? {
        guard fact.isBool, fact.enumValues.indices.contains(index) else { return nil }
        return fact.enumValues[index] == "true" ? true : fact.enumValues[index] == "false" ? false : nil
    }

    private var selected: Int {
        fact.enumStrings.indices.first { index in boolOption(index).map { $0 == fact.boolValue } ?? (index == fact.enumIndex) } ?? -1
    }

    private func pick(_ index: Int) {
        let path = fact.path, inverted = fact.inverted, option = boolOption(index)
        write { option.map { Qgc.set(path, $0 != inverted) } ?? Qgc.set("\(path).enumIndex", index) }
    }

    var body: some View {
        Picker(fact.title, selection: Binding(get: { selected }, set: pick)) {
            ForEach(Array(fact.enumStrings.enumerated()), id: \.offset) { index, option in
                Text(sentenceCase(option)).lineLimit(1).tag(index)
            }
        }
        .pickerStyle(.segmented)
        .labelsHidden()
    }
}

let SEGMENT_LABEL_BUDGET = 28
let SEGMENTS_BESIDE_MIN_WIDTH: CGFloat = 560
let NUMBER_VALUE_WIDTH: CGFloat = 160
let CHOICE_VALUE_WIDTH: CGFloat = 220
private let TRAILING_MAX_WIDTH: CGFloat = 190

private let SECONDS: Set<String> = ["s", "sec", "secs", "second", "seconds"]

func shownUnits(_ units: String) -> String { SECONDS.contains(units.lowercased()) ? "s" : units }

func showsAsField(_ fact: Fact) -> Bool { !fact.readOnly && !editOnDesktop(fact) && !fact.isBool }

func valueOnTheRight(_ fact: Fact) -> Bool { !fact.isString && !fact.isBitmask }

func showsAsSegments(_ isEnum: Bool, _ offList: Bool, _ writable: Bool, _ options: [String]) -> Bool {
    isEnum && !offList && writable && (2...4).contains(options.count) && options.map(\.utf16.count).reduce(0, +) <= SEGMENT_LABEL_BUDGET
}

func bitmaskToggled(_ fact: Fact, _ raw: Int64, _ index: Int) -> Int64 {
    let bit = fact.bitmaskValues[index]
    if raw & bit != 0 { return raw & ~bit }
    if fact.firstEntryIsAll && index == 0 { return fact.bitmaskValues.dropFirst().reduce(raw) { value, other in value & ~other } | bit }
    return raw | bit
}

func bitmaskEntryEnabled(_ fact: Fact, _ raw: Int64, _ index: Int) -> Bool {
    !(fact.firstEntryIsAll && index > 0 && !fact.bitmaskValues.isEmpty && raw & fact.bitmaskValues[0] != 0)
}

private struct BitmaskPicker: View {
    let fact: Fact
    let write: (@escaping @Sendable () -> Bool) -> Void
    @Environment(\.theme) private var theme
    @State private var editing = false

    private func flip(_ index: Int) {
        let path = fact.path, next = bitmaskToggled(fact, bitmaskRaw(fact), index)
        write { Qgc.set(path, String(next)) }
    }

    var body: some View {
        Button { editing = true } label: {
            HStack {
                Text(bitmaskSummary(fact)).lineLimit(3).frame(maxWidth: .infinity, alignment: .leading)
                Image(.arrowDropDown)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 14)
            .overlay(RoundedRectangle(cornerRadius: Corner.extraSmall).stroke(theme.colors.outline))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .sheet(isPresented: $editing) {
            NavigationStack {
                List(fact.bitmaskStrings.indices.filter { fact.bitmaskValues.indices.contains($0) }, id: \.self) { index in
                    let raw = bitmaskRaw(fact)
                    let checked = raw & fact.bitmaskValues[index] != 0
                    Button { flip(index) } label: {
                        HStack(spacing: 12) {
                            Image(systemName: checked ? "checkmark.square.fill" : "square")
                                .foregroundStyle(checked ? theme.colors.primary : theme.colors.onSurfaceVariant)
                            Text(fact.bitmaskStrings[index]).frame(maxWidth: .infinity, alignment: .leading)
                        }
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                    .disabled(!(fact.enabled && bitmaskEntryEnabled(fact, raw, index)))
                }
                .navigationTitle(fact.title)
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .confirmationAction) { Button("Done") { editing = false } }
                }
            }
            .presentationDetents([.medium, .large])
            .presentationDragIndicator(.visible)
        }
    }
}

private func chooser(_ fact: Fact) -> @Sendable (Int) -> Bool {
    let path = fact.path, values = fact.enumValues, raw = fact.rawChoice
    return { index in
        let rawValue = raw && values.indices.contains(index) ? Int64(values[index]) : nil
        return rawValue.map { Qgc.set(path, $0) } ?? Qgc.set("\(path).enumIndex", index)
    }
}

private struct EnumField: View {
    let fact: Fact
    let write: (@escaping @Sendable () -> Bool) -> Void
    @Environment(\.LocalChangeNotice) private var notice

    private func pick(_ index: Int) {
        let previous = fact.enumIndex
        let choose = chooser(fact)
        let message = "\(sentenceCase(fact.heading)): \(sentenceCase(fact.enumStrings.indices.contains(index) ? fact.enumStrings[index] : ""))"
        let notice = notice, write = write
        write {
            let accepted = choose(index)
            if accepted && index != previous && previous >= 0 {
                onMain {
                    notice.show(message) {
                        await MainActor.run { write { choose(previous) } }
                        return nil
                    }
                }
            }
            return accepted
        }
    }

    var body: some View {
        ChoiceField(label: nil, value: shownEnumLabel(fact), options: fact.enumStrings.map(sentenceCase), enabled: fact.enabled, groups: fact.enumGroups, onPick: pick)
    }
}

let ADVANCED_SAFETY = "Advanced safety settings"

func rowChoiceLabel(_ label: String) -> String { label.components(separatedBy: ", ").first ?? label }

private let DISABLED_VALUE_ALPHA = 0.5

private struct OptionRun: Identifiable {
    let id: Int
    let group: String
    let indices: Range<Int>
}

private func optionRuns(_ count: Int, _ groups: [String]) -> [OptionRun] {
    let groupAt: (Int) -> String = { groups.indices.contains($0) ? groups[$0] : "" }
    let starts = (0..<count).filter { $0 == 0 || groupAt($0) != groupAt($0 - 1) }
    return starts.enumerated().map { at, start in
        OptionRun(id: start, group: groupAt(start), indices: start..<(at + 1 < starts.count ? starts[at + 1] : count))
    }
}

struct ChoiceField: View {
    let label: String?
    let value: String
    let options: [String]
    var enabled: Bool = true
    var groups: [String] = []
    let onPick: (Int) -> Void
    @Environment(\.theme) private var theme
    @Environment(\.LocalSettingsList) private var settingsList

    var body: some View {
        if settingsList && label == nil {
            SafeChoiceMenu(options: options, onPick: onPick) {
                HStack(spacing: 0) {
                    Text(rowChoiceLabel(value))
                        .font(.bodyLarge)
                        .foregroundStyle(theme.colors.onSurfaceVariant.opacity(enabled ? 1 : DISABLED_VALUE_ALPHA))
                        .lineLimit(1)
                    Image(.chevronRight).foregroundStyle(theme.colors.onSurfaceVariant)
                }
                .frame(maxWidth: .infinity, minHeight: 48, alignment: .trailing)
                .contentShape(Rectangle())
            }
            .disabled(!enabled)
        } else {
            Menu {
                ForEach(optionRuns(options.count, groups)) { run in
                    if run.group.isBlank {
                        ForEach(run.indices, id: \.self) { index in Button(options[index]) { onPick(index) } }
                    } else {
                        Section(run.group) {
                            ForEach(run.indices, id: \.self) { index in Button(options[index]) { onPick(index) } }
                        }
                    }
                }
            } label: {
                VStack(alignment: .leading, spacing: 2) {
                    if let label {
                        Text(label).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant).lineLimit(1)
                    }
                    HStack {
                        Text(value).font(.bodyLarge).foregroundStyle(theme.colors.onSurface).lineLimit(1).frame(maxWidth: .infinity, alignment: .leading)
                        Image(.arrowDropDown).foregroundStyle(theme.colors.onSurfaceVariant)
                    }
                }
                .padding(.horizontal, 16)
                .padding(.vertical, 10)
                .overlay(RoundedRectangle(cornerRadius: Corner.extraSmall).stroke(theme.colors.outline))
                .opacity(enabled ? 1 : DISABLED_VALUE_ALPHA)
                .contentShape(Rectangle())
            }
            .disabled(!enabled)
        }
    }
}

func factValueLines(_ fact: Fact) -> Int { fact.isString ? 4 : 1 }

func truncationRefusal(_ fact: Fact, _ text: String) -> String? {
    guard fact.wholeNumbersOnly, let typed = toDoubleOrNull(text), typed.isFinite else { return nil }
    return typed == typed.rounded(.down) ? nil : "Invalid number"
}

func typedValue(_ text: String) -> String { text.replacingOccurrences(of: "\n", with: "") }

let ASPECT_RATIO = "aspectRatio"
private let RATIO_TOLERANCE = 0.005
private let COMMON_RATIOS = [(16, 9), (4, 3), (21, 9), (16, 10), (3, 2), (5, 4), (1, 1)]

func ratioText(_ value: Double) -> String? {
    COMMON_RATIOS.first { abs(Double($0.0) / Double($0.1) - value) < RATIO_TOLERANCE }.map { "\($0.0):\($0.1)" }
}

func ratioValue(_ text: String) -> String? {
    let parts = text.components(separatedBy: ":")
    guard parts.count == 2, let width = toDoubleOrNull(parts[0]), let height = toDoubleOrNull(parts[1]), width > 0, height > 0 else { return nil }
    return String(format: "%.6f", width / height)
}

func isAddress(_ fact: Fact) -> Bool { fact.isString && (fact.name.hasSuffix("Url") || fact.name.hasSuffix("URL")) }

func factKeyboard(_ fact: Fact) -> UIKeyboardType {
    let min = toDoubleOrNull(fact.minString)
    if isAddress(fact) { return .URL }
    if fact.isString || fact.isBool { return .default }
    if fact.wholeNumbersOnly, let min, min >= 0 { return .numberPad }
    if min.map({ $0 < 0 }) != false { return .numbersAndPunctuation }
    return .decimalPad
}

private let KEYBOARDS_WITHOUT_RETURN: Set<UIKeyboardType> = [.numberPad, .decimalPad]

func fieldText(_ fact: Fact) -> String { fact.isString || isSecret(fact) ? fact.valueString : plainNumber(fact.valueString) }

func plainNumber(_ text: String) -> String {
    guard let parsed = toDoubleOrNull(text), parsed.isFinite, abs(parsed) < 1e15 else { return text }
    return parsed == parsed.rounded(.down) ? String(Int64(parsed)) : NSDecimalNumber(string: String(parsed)).stringValue
}

func factConstraintNote(_ fact: Fact) -> String? {
    let parts = [
        !fact.minString.isBlank && !fact.minIsDefaultForType ? "Min \(plainNumber(fact.minString))" : nil,
        !fact.maxString.isBlank && !fact.maxIsDefaultForType ? "Max \(plainNumber(fact.maxString))" : nil,
        !fact.defaultValueString.isBlank ? "Default \(plainNumber(fact.defaultValueString))" : nil,
    ].compactMap { $0 }
    return parts.isEmpty ? nil : parts.joined(separator: " · ")
}

func factRebootNote(_ fact: Fact) -> String? {
    fact.vehicleRebootRequired ? "Reboot vehicle for changes to take effect."
        : fact.qgcRebootRequired ? "Restart Aircast for this to take effect."
        : nil
}

func writeRefusal(_ accepted: Bool) -> String? { accepted ? nil : "That change was not accepted." }

func validationMessage(_ result: JSON?) -> String? {
    if case .string(let text) = result, !text.isBlank { return text }
    return nil
}

private func blockingRejection(_ fact: Fact, _ text: String) -> String? {
    truncationRefusal(fact, text) ?? validationMessage(Qgc.invokeResult("\(fact.path).validate", text, false))
}

private func rejectionFor(_ fact: Fact, _ text: String) async -> String? {
    await offMain { blockingRejection(fact, text) }
}

func storedText(_ fact: Fact, _ typed: String) -> String {
    fact.name == ASPECT_RATIO ? ratioValue(typed) ?? typed : typed
}

func shownText(_ fact: Fact) -> String {
    fact.name == ASPECT_RATIO ? toDoubleOrNull(fact.valueString).flatMap(ratioText) ?? fieldText(fact) : fieldText(fact)
}

private struct FactTextField: View {
    let fact: Fact
    let onWrite: () -> Void
    var onRejected: () -> Void = {}
    var onTheRight: Bool = false
    @Environment(\.theme) private var theme
    @Environment(\.LocalBlockRebootNote) private var blockRebootNote
    @State private var editing: String? = nil
    @State private var rejection: String? = nil
    @State private var revealed = false
    @FocusState private var focused: Bool

    private var pending: String? { editing.flatMap { $0 != shownText(fact) ? $0 : nil } }

    private var text: Binding<String> {
        Binding(
            get: { editing ?? shownText(fact) },
            set: { typed in
                editing = typedValue(typed)
                rejection = nil
                if typed.contains("\n") { focused = false }
            }
        )
    }

    private func commit() {
        guard let typed = pending else { return }
        let committed = storedText(fact, typed)
        let fact = fact, onWrite = onWrite, onRejected = onRejected
        Task { @MainActor in
            if let refused = await rejectionFor(fact, committed) {
                rejection = refused
                onRejected()
                return
            }
            let path = fact.path
            let refusal = await offMain { Qgc.writeRefusal(path, committed) }
            rejection = refusal
            if refusal == nil {
                editing = nil
                onWrite()
            }
        }
    }

    var body: some View {
        let secret = isSecret(fact)
        let lines = factValueLines(fact)
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 8) {
                if secret {
                    Button(revealed ? "Hide" : "Show") { revealed.toggle() }.buttonStyle(.borderless)
                }
                Group {
                    if secret && !revealed {
                        SecureField("", text: text)
                    } else {
                        TextField("", text: text, axis: lines == 1 ? .horizontal : .vertical)
                            .lineLimit(1...lines)
                    }
                }
                .multilineTextAlignment(onTheRight ? .trailing : .leading)
                .keyboardType(factKeyboard(fact))
                .autocorrectionDisabled(isAddress(fact))
                .textInputAutocapitalization(isAddress(fact) || !fact.isString ? .never : nil)
                .submitLabel(.done)
                .onSubmit { focused = false }
                .focused($focused)
                .disabled(!fact.enabled)
                .toolbar {
                    if focused && KEYBOARDS_WITHOUT_RETURN.contains(factKeyboard(fact)) {
                        ToolbarItemGroup(placement: .keyboard) {
                            Spacer()
                            Button("Done") { focused = false }
                        }
                    }
                }
                if !fact.units.isBlank {
                    Text(shownUnits(fact.units)).foregroundStyle(theme.colors.onSurfaceVariant)
                }
                if pending != nil {
                    Button("Set", action: commit).buttonStyle(.borderless)
                }
            }
            .padding(12)
            .overlay(
                RoundedRectangle(cornerRadius: Corner.extraSmall)
                    .stroke(rejection != nil ? theme.colors.error : focused ? theme.colors.primary : theme.colors.outline, lineWidth: focused ? 2 : 1)
            )
            .opacity(fact.enabled ? 1 : 0.38)
            if let rejection {
                Text(rejection).font(.bodySmall).foregroundStyle(theme.colors.error).padding(.leading, 16).padding(.top, 4)
            }
            if rejection == nil && !fact.problem.isBlank && (editing ?? "").isBlank {
                Text(fact.problem).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant).padding(.leading, 16).padding(.top, 4)
            }
            if rejection == nil && focused, let note = factConstraintNote(fact) {
                Text(note).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant).padding(.leading, 16).padding(.top, 4)
            }
            if let reboot = factRebootNote(fact), reboot != blockRebootNote {
                Text(reboot).font(.bodySmall).foregroundStyle(theme.aircast.warning).padding(.leading, 16).padding(.top, 4)
            }
        }
        .frame(width: onTheRight ? NUMBER_VALUE_WIDTH : nil)
        .frame(maxWidth: onTheRight ? nil : .infinity, alignment: .leading)
        .onChange(of: focused) { if !focused { commit() } }
        .onChange(of: fact.path) {
            editing = nil
            rejection = nil
            revealed = false
        }
        .onDisappear {
            guard let typed = pending else { return }
            let committed = storedText(fact, typed)
            let fact = fact
            offMain { if blockingRejection(fact, committed) == nil { _ = Qgc.writeRefusal(fact.path, committed) } }
        }
    }
}
