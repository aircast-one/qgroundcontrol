#if os(macOS)
import Network
import SwiftUI

// Writes Aircast OS to a drone's SD card, as Android's FlasherScreen does. iOS gives apps no raw disk access, so the
// page is Mac-only. Android also pre-fills the WiFi network with the phone's own SSID; a Mac reads its SSID only with
// Location Services granted, and the network is typed in a few keystrokes, so that convenience is left out here.
//
// Not yet reachable from Settings: the core writes cards through platformdisk, which has a backend on Windows and
// host hooks on Android but none on macOS yet, so view.flasher reports available=false here. Writing a raw disk on
// macOS needs administrator rights (authopen or a privileged helper). Add the Settings entry with that backend.

let FLASHER_PAGE = "Aircast card"
let FLASHER_VIEW = "view.flasher"
let FLASHER_OPEN = "flasher.open"
let FLASHER_CHANNEL = "flasher.channel"
let FLASHER_SELECT = "flasher.select"
let FLASHER_DOWNLOAD = "flasher.download"
let FLASHER_FORM = "flasher.form"
let FLASHER_DISKS = "flasher.disks"
let FLASHER_CHOOSE_DISK = "flasher.chooseDisk"
let FLASHER_START = "flasher.start"
let FLASHER_CANCEL = "flasher.cancel"
let FLASHER_AGAIN = "flasher.again"
let FLASHER_POLL_MS = 500
let FLASHER_DISK_POLL_MS = 2_000
let FLASHER_PASSWORD_MODE = "password"
let FLASHER_FINISHED: Set<String> = ["done", "failed", "cancelled"]
let FLASHER_SETUP_FIELDS: Set<String> = ["hostname", "ssid", "wifiPassword", "devicePassword", "authorizedKey", "controlServer"]
let FLASHER_MORE_FIELDS: Set<String> = ["authorizedKey", "controlServer"]
let FLASHER_FIELD_LABELS: [String: String] = [
    "hostname": "Drone name",
    "ssid": "WiFi network",
    "wifiPassword": "WiFi password",
    "devicePassword": "Password for pi",
    "authorizedKey": "SSH public key",
    "controlServer": "Headscale server",
]
let FLASHER_MORE_ROWS: Set<String> = ["Image", "Remote access"]
let FLASHER_BLOCK = "Wait for the card to finish writing or cancel it first"
let FLASHER_SSH_MODES: [(first: String, second: String)] = [("password", "Password"), ("key-only", "SSH key"), ("disabled", "Off")]
private let FLASHER_AWAKE = "Writing an Aircast card"

#if DEBUG
private let FLASHER_DEBUG_BUILD = true
#else
private let FLASHER_DEBUG_BUILD = false
#endif

enum FlasherStep: Int, CaseIterable {
    case Card, Setup, Write

    var title: String {
        switch self {
        case .Card: "Card"
        case .Setup: "Setup"
        case .Write: "Write"
        }
    }
}

struct FlasherRelease: Equatable {
    let version: String
    let label: String
    let published: String
    let sizeText: String
    var recommended: Bool = false
}

struct FlasherDisk: Equatable, Identifiable {
    let id: String
    let name: String
}

struct FlasherSummaryRow: Equatable {
    let label: String
    let value: String
    let warn: Bool
}

struct FlasherForm: Equatable {
    var hostname = ""
    var ssid = ""
    var wifiPassword = ""
    var noWifi = false
    var country = ""
    var sshMode = "key-only"
    var authorizedKey = ""
    var devicePassword = ""
    var authKey = ""
    var controlServer = ""
}

struct FlasherState: Equatable {
    var available: Bool
    var channel: String
    var channels: [String]
    var releasesState: String
    var releasesError: String
    var releases: [FlasherRelease]
    var selected: String
    var form: FlasherForm
    var problems: [String: String]
    var keyIdentity: String
    var summary: [FlasherSummaryRow]
    var disks: [FlasherDisk]
    var cardState: String
    var cardId: String
    var cardText: String
    var downloadState: String
    var downloadText: String
    var phase: String
    var busy: Bool
    var percent: Double
    var speedText: String
    var jobError: String
    var hostname: String
    var canStart: Bool
    var blocked: String
    var firstProblemField: String = ""
    var firstProblem: String = ""
    var remainingText: String = ""
}

func flasherForm(_ json: JSON?) -> FlasherForm {
    guard let json, json.object != nil else { return FlasherForm() }
    return FlasherForm(
        hostname: json["hostname"].string,
        ssid: json["ssid"].string,
        wifiPassword: json["wifiPassword"].string,
        noWifi: json["noWifi"].bool,
        country: json["country"].string,
        sshMode: json["sshMode"].string.ifBlank("key-only"),
        authorizedKey: json["authorizedKey"].string,
        devicePassword: json["devicePassword"].string,
        authKey: json["authKey"].string,
        controlServer: json["controlServer"].string
    )
}

func flasherCardText(_ card: JSON?) -> String {
    guard let card else { return "" }
    switch card["state"].string {
    case "opening": return "Checking the card reader…"
    case "waiting": return "Allow Aircast to use the card reader when asked"
    case "ready": return [card["label"].string.ifBlank("Card reader"), card["capacityText"].string].filter { !$0.isBlank }.joined(separator: " · ")
    case "failed": return card["error"].string
    default: return ""
    }
}

func flasherDownloadText(_ download: JSON?) -> String {
    guard let download else { return "" }
    switch download["state"].string {
    case "running": return "Downloading Aircast OS… \(saturatingInt(download["percent"].double(0)))%"
    case "ready": return "Aircast OS is downloaded"
    case "failed": return "Download failed: \(download["error"].string)"
    default: return ""
    }
}

func flasherState(_ json: JSON?) -> FlasherState? {
    guard let view = json, view.has("job") else { return nil }
    let releases = view["releases"]
    let identity = view["keyIdentity"]
    let job = view["job"]
    let card = view["card"]
    let download = view["download"]
    return FlasherState(
        available: view["available"].bool,
        channel: view["channel"].string,
        channels: view["channels"].strings,
        releasesState: releases["state"].string,
        releasesError: releases["error"].string,
        releases: releases["items"].objects.map {
            FlasherRelease(version: $0["version"].string, label: $0["label"].string, published: $0["published"].string, sizeText: $0["sizeText"].string, recommended: $0["recommended"].bool)
        },
        selected: view["selected"].string,
        form: flasherForm(view["form"]),
        problems: (view["problems"].object ?? [:]).mapValues(\.string),
        keyIdentity: identity.object == nil ? "" : [identity["algorithm"].string, identity["fingerprint"].string, identity["comment"].string].filter { !$0.isBlank }.joined(separator: " "),
        summary: view["summary"].objects.map { FlasherSummaryRow(label: $0["label"].string, value: $0["value"].string, warn: $0["warn"].bool) },
        disks: view["disks"].objects.map { FlasherDisk(id: $0["id"].string, name: $0["name"].string) },
        cardState: card["state"].string,
        cardId: card["id"].string,
        cardText: flasherCardText(card.object == nil ? nil : card),
        downloadState: download["state"].string,
        downloadText: flasherDownloadText(download.object == nil ? nil : download),
        phase: job["phase"].string,
        busy: job["busy"].bool,
        percent: job["percent"].double(0),
        speedText: job["speedText"].string,
        jobError: job["error"].string,
        hostname: job["hostname"].string,
        canStart: view["canStart"].bool,
        blocked: view["blocked"].string,
        firstProblemField: view["firstProblem"]["field"].string,
        firstProblem: view["firstProblem"]["message"].string,
        remainingText: job["remainingText"].string
    )
}

func flasherPhaseLabel(_ phase: String) -> String {
    switch phase {
    case "downloading": "Downloading Aircast OS"
    case "preparing": "Preparing the card"
    case "writing": "Writing the card"
    case "verifying": "Checking what was written"
    case "customizing": "Setting up WiFi and access"
    case "done": "Card ready"
    case "failed": "Writing failed"
    case "cancelled": "Cancelled"
    default: ""
    }
}

func flasherStepFor(_ state: FlasherState?) -> FlasherStep? {
    guard let state, state.busy || FLASHER_FINISHED.contains(state.phase) else { return nil }
    return .Write
}

func flasherReaderToOpen(_ state: FlasherState) -> String? {
    guard state.disks.count == 1, state.cardState == "none", !state.busy, !FLASHER_FINISHED.contains(state.phase) else { return nil }
    return state.disks[0].id
}

func flasherVisibleProblems(_ problems: [String: String], _ touched: Set<String>, _ showAll: Bool) -> [String: String] {
    problems.filter { showAll || touched.contains($0.key) }
}

func flasherSetupComplete(_ problems: [String: String]) -> Bool {
    !problems.keys.contains { FLASHER_SETUP_FIELDS.contains($0) }
}

func flasherDefaultSshMode(_ form: FlasherForm) -> String? {
    form.sshMode == "key-only" && form.authorizedKey.isBlank ? FLASHER_PASSWORD_MODE : nil
}

func flasherMeteredNote(_ release: FlasherRelease?, _ downloadState: String, _ metered: Bool) -> String? {
    guard let release, metered, downloadState != "ready", downloadState != "running" else { return nil }
    return "Aircast OS is a \(release.sizeText) download and this Mac is on a metered connection."
}

func flasherProblemNote(_ state: FlasherState) -> String {
    guard !state.firstProblem.isBlank else { return "" }
    return FLASHER_FIELD_LABELS[state.firstProblemField].map { "\($0): \(state.firstProblem)" } ?? state.firstProblem
}

func flasherProblemInMoreOptions(_ state: FlasherState) -> Bool { FLASHER_MORE_FIELDS.contains(state.firstProblemField) }

func flasherPreviousStep(_ step: FlasherStep, _ busy: Bool, _ phase: String) -> FlasherStep? {
    guard step != .Card, !busy, !FLASHER_FINISHED.contains(phase) else { return nil }
    return FlasherStep(rawValue: step.rawValue - 1)
}

func flasherShowsChannels(_ debug: Bool, _ channel: String) -> Bool { debug || channel != "stable" }

func flasherKeepsAwake(_ state: FlasherState?) -> Bool {
    guard let state else { return false }
    return state.busy || state.downloadState == "running"
}

func flasherNextSteps(_ ssid: String, _ hostname: String, _ noWifi: Bool) -> [String] {
    [
        "Unplug the reader, put the card in the drone and power it on.",
        noWifi
            ? "Give it about 3 minutes for its first boot on Ethernet or cellular."
            : "Give it about 3 minutes for its first boot. It joins \(ssid) by itself.",
        hostname.isBlank
            ? "Find it on your network under the image's default name."
            : "Open http://\(hostname).local on a phone or computer on the same network to see the drone and its links.",
    ]
}

func flasherReleaseDetail(_ release: FlasherRelease) -> String {
    [release.published, "\(release.sizeText) download", release.recommended ? "Recommended" : ""].filter { !$0.isBlank }.joined(separator: " · ")
}

func flasherActionNote(_ step: FlasherStep, _ state: FlasherState, _ refusal: String, _ setupNote: String) -> String {
    if !refusal.isBlank { return refusal }
    if step == .Setup && !setupNote.isBlank { return setupNote }
    if step == .Write && !state.busy && !FLASHER_FINISHED.contains(state.phase) { return state.blocked }
    return ""
}

private func flasherPatch(_ key: String, _ value: JSON) {
    offMainInOrder { _ = Qgc.refusalOf(FLASHER_FORM, JSON.object([key: value])) }
}

struct FlasherScreen: View {
    @State private var more = false
    @State private var state: FlasherState?
    @State private var form: FlasherForm?
    @State private var step = FlasherStep.Card
    @State private var touched: Set<String> = []
    @State private var showAllProblems = false
    @State private var refusal = ""
    @State private var metered = false
    @State private var scope = ViewScope()
    @Environment(\.theme) private var theme

    // The three steps as a native form: the step strip, the step's sections, its note, and its next action as the
    // page's primary action; the navigation bar's back steps back through the steps first.
    var body: some View {
        let back = flasherPreviousStep(step, state?.busy == true, state?.phase ?? "")
        VStack(spacing: 0) {
            if let current = state, let fields = form {
                if current.available {
                    NativeForm {
                        Section { stepHeader(back) }
                        stepBody(current, fields)
                        actions(current)
                    }
                    .toolbar {
                        ToolbarItem(placement: .confirmationAction) { primaryAction(current) }
                    }
                } else {
                    Text("This device cannot write cards.").foregroundStyle(theme.aircast.warning).padding(Space.s4)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
            } else {
                Text("Loading…").padding(Space.s4).frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .background { BlocksNavigation(blocking: state?.busy == true, reason: FLASHER_BLOCK) }
        .background {
            if let back { OverridePageHeading(title: FLASHER_PAGE, onBack: { step = back }) }
        }
        .task { await poll() }
        .task {
            while !Task.isCancelled {
                try? await Task.sleep(for: .milliseconds(FLASHER_DISK_POLL_MS))
                _ = await offMain { Qgc.invoke(FLASHER_DISKS) }
            }
        }
        .task { await watchNetworkCost() }
        .onChange(of: flasherKeepsAwake(state), initial: true) { _, awake in
            if awake {
                MacActivity.begin(FLASHER_AWAKE, keepDisplayOn: true)
            } else {
                MacActivity.end(FLASHER_AWAKE)
            }
        }
        .onDisappear {
            MacActivity.end(FLASHER_AWAKE)
            scope.cancel()
            if state?.busy == true { offMainInOrder { _ = Qgc.invoke(FLASHER_CANCEL) } }
        }
    }

    @MainActor
    private func poll() async {
        _ = await offMain { Qgc.invoke(FLASHER_OPEN) }
        while !Task.isCancelled {
            let read = await offMain { flasherState(Qgc.get(FLASHER_VIEW)) }
            state = read
            if let next = flasherStepFor(read) { step = next }
            if form == nil, let read {
                let country = read.form.country.ifBlank(Locale.current.region?.identifier ?? "")
                let sshMode = flasherDefaultSshMode(read.form) ?? read.form.sshMode
                if country != read.form.country { flasherPatch("country", .string(country)) }
                if sshMode != read.form.sshMode { flasherPatch("sshMode", .string(sshMode)) }
                form = withChanges(read.form) {
                    $0.country = country
                    $0.sshMode = sshMode
                }
            }
            if let read, let reader = flasherReaderToOpen(read) {
                _ = await offMain { Qgc.invoke(FLASHER_CHOOSE_DISK, reader) }
            }
            try? await Task.sleep(for: .milliseconds(FLASHER_POLL_MS))
        }
    }

    // A Mac on a phone hotspot or a metered network is asked before the image download starts, as Android asks on
    // mobile data.
    @MainActor
    private func watchNetworkCost() async {
        let monitor = NWPathMonitor()
        let costs = AsyncStream<Bool> { continuation in
            monitor.pathUpdateHandler = { path in continuation.yield(path.isExpensive || path.isConstrained) }
            continuation.onTermination = { _ in monitor.cancel() }
            monitor.start(queue: DispatchQueue(label: "one.aircast.flasher.network"))
        }
        for await expensive in costs {
            metered = expensive
        }
    }

    private func command(_ path: String, _ args: [String] = []) {
        scope.launch {
            refusal = await offMain { Qgc.refusalOf(path, arguments: args) } ?? ""
        }
    }

    private func edit(_ next: FlasherForm, _ key: String, _ value: JSON) {
        form = next
        touched.insert(key)
        flasherPatch(key, value)
    }

    private func advance(_ current: FlasherState) {
        refusal = ""
        switch step {
        case .Card:
            step = .Setup
            if !metered && ["idle", "failed", ""].contains(current.downloadState) { command(FLASHER_DOWNLOAD) }
        case .Setup:
            showAllProblems = true
            if flasherProblemInMoreOptions(current) { more = true }
            if flasherSetupComplete(current.problems) { step = .Write }
        case .Write:
            command(FLASHER_START)
        }
    }

    private func again(_ finished: String) {
        scope.launch {
            let read = await offMain { () -> FlasherState? in
                _ = Qgc.refusalOf(FLASHER_AGAIN)
                return flasherState(Qgc.get(FLASHER_VIEW))
            }
            if let read {
                form = read.form
                state = read
            }
            touched = []
            showAllProblems = false
            step = finished == "done" ? .Card : .Write
        }
    }

    private func stepHeader(_ back: FlasherStep?) -> some View {
        HStack(spacing: 0) {
            ForEach(FlasherStep.allCases, id: \.self) { each in
                let done = each.rawValue < step.rawValue
                let mark = done ? "✓" : String(each.rawValue + 1)
                Button {
                    if back != nil && done { step = each }
                } label: {
                    Text("\(mark)  \(each.title)")
                        .font(.labelLarge)
                        .fontWeight(each == step ? .bold : .regular)
                        .foregroundStyle(each.rawValue <= step.rawValue ? theme.colors.primary : theme.colors.onSurfaceVariant)
                        .padding(Space.s2)
                        .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .allowsHitTesting(done)
                if each != .Write { Spacer(minLength: Space.s2) }
            }
        }
        .padding(.horizontal, Space.s2)
        .padding(.vertical, Space.s1)
    }

    @ViewBuilder
    private func actions(_ current: FlasherState) -> some View {
        let note = flasherActionNote(step, current, refusal, showAllProblems ? flasherProblemNote(current) : "")
        if !note.isBlank {
            FormNote(text: note, color: theme.aircast.warning)
        }
        if current.busy {
            Section {
                Button("Cancel writing", role: .destructive) { command(FLASHER_CANCEL) }
            }
        }
    }

    @ViewBuilder
    private func primaryAction(_ current: FlasherState) -> some View {
        if current.busy {
            EmptyView()
        } else if FLASHER_FINISHED.contains(current.phase) {
            Button(current.phase == "done" ? "Flash another card" : "Try again") { again(current.phase) }
        } else {
            Button(step == .Write ? "Write card" : "Continue") { advance(current) }
                .disabled(step == .Write && !current.canStart)
        }
    }

    @ViewBuilder
    private func stepBody(_ current: FlasherState, _ fields: FlasherForm) -> some View {
        let release = current.releases.first { $0.version == current.selected }
        let meteredNote = flasherMeteredNote(release, current.downloadState, metered)
        switch step {
        case .Card: cardStep(current)
        case .Setup: setupStep(current, fields, meteredNote)
        case .Write: writeStep(current, fields, meteredNote)
        }
    }

    private func heading(_ title: String, _ detail: String) -> some View {
        Section {
            VStack(alignment: .leading, spacing: Space.s1) {
                Text(title).font(.headline)
                Text(detail).font(.subheadline).foregroundStyle(Color.secondary)
            }
            .padding(.vertical, Space.s1)
        }
    }

    private func choiceRow(_ title: String, _ detail: String, selected: Bool, detailColor: Color? = nil, trailing: Bool = false, onClick: @escaping () -> Void) -> some View {
        Button(action: onClick) {
            HStack(spacing: Space.s3) {
                VStack(alignment: .leading, spacing: 2) {
                    Text(title).foregroundStyle(Color.primary)
                    if !detail.isBlank {
                        Text(detail).font(.subheadline).foregroundStyle(detailColor ?? Color.secondary)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                if selected {
                    Image(systemName: trailing ? "checkmark.circle.fill" : "checkmark")
                        .fontWeight(.semibold)
                        .foregroundStyle(trailing ? theme.aircast.success : theme.colors.primary)
                }
            }
            .contentShape(Rectangle())
        }
        .listRowButton()
        .accessibilityAddTraits(selected ? .isSelected : [])
    }

    @ViewBuilder
    private func cardStep(_ current: FlasherState) -> some View {
        heading("Insert the card", "Put the drone's SD card in this Mac's card slot, or in a USB card reader plugged into it.")
        Section {
            if current.disks.isEmpty {
                VStack(alignment: .leading, spacing: 2) {
                    Text("Waiting for a card reader…")
                    Text("You can continue and plug it in before writing").font(.subheadline).foregroundStyle(Color.secondary)
                }
            }
            ForEach(current.disks) { disk in
                let chosen = disk.id == current.cardId
                choiceRow(
                    disk.name,
                    chosen ? current.cardText : "",
                    selected: chosen,
                    detailColor: current.cardState == "failed" ? theme.aircast.warning : nil,
                    trailing: chosen && current.cardState == "ready"
                ) { command(FLASHER_CHOOSE_DISK, [disk.id]) }
            }
            if current.cardState == "failed" && !current.cardId.isBlank {
                Button("Try the reader again") { command(FLASHER_CHOOSE_DISK, [current.cardId]) }
            }
        } header: {
            SectionHeader(text: "Card reader")
        }
    }

    @ViewBuilder
    private func setupStep(_ current: FlasherState, _ fields: FlasherForm, _ meteredNote: String?) -> some View {
        let problems = flasherVisibleProblems(current.problems, touched, showAllProblems)
        heading("Set up the drone", "These are applied on the drone's first boot, so it joins your network by itself.")
        Section {
            FlasherField(label: "Drone name", value: fields.hostname, problem: problems["hostname"], help: fields.hostname.isBlank ? nil : "It answers at \(fields.hostname).local") { typed in
                edit(withChanges(fields) { $0.hostname = typed }, "hostname", .string(typed))
            }
            if !fields.noWifi {
                FlasherField(label: "WiFi network", value: fields.ssid, problem: problems["ssid"]) { typed in
                    edit(withChanges(fields) { $0.ssid = typed }, "ssid", .string(typed))
                }
                FlasherField(label: "WiFi password", value: fields.wifiPassword, problem: problems["wifiPassword"], secret: true) { typed in
                    edit(withChanges(fields) { $0.wifiPassword = typed }, "wifiPassword", .string(typed))
                }
            }
            Toggle(isOn: Binding(get: { fields.noWifi }, set: { checked in
                edit(withChanges(fields) { $0.noWifi = checked }, "noWifi", .bool(checked))
            })) {
                VStack(alignment: .leading, spacing: 2) {
                    Text("No WiFi")
                    Text("The drone goes online over Ethernet or cellular").font(.subheadline).foregroundStyle(Color.secondary)
                }
            }
            if fields.sshMode == FLASHER_PASSWORD_MODE {
                FlasherField(label: "Password for pi", value: fields.devicePassword, problem: problems["devicePassword"], help: "At least 8 characters. You sign in to the drone as pi with it.", secret: true) { typed in
                    edit(withChanges(fields) { $0.devicePassword = typed }, "devicePassword", .string(typed))
                }
            }
        }
        downloadLine(current, meteredNote)
        Section {
            AdvancedToggle(open: more, label: "More options") { more.toggle() }
        }
        if more {
            moreOptions(current, fields, problems)
        }
    }

    @ViewBuilder
    private func downloadLine(_ current: FlasherState, _ meteredNote: String?) -> some View {
        if let meteredNote {
            Section {
                Text(meteredNote).font(.footnote).foregroundStyle(theme.aircast.warning)
                Button("Download now anyway") { command(FLASHER_DOWNLOAD) }
            }
        } else if !current.downloadText.isBlank {
            FormNote(text: current.downloadText)
        }
    }

    @ViewBuilder
    private func moreOptions(_ current: FlasherState, _ fields: FlasherForm, _ problems: [String: String]) -> some View {
        Section {
            ForEach(current.releases, id: \.version) { release in
                choiceRow(release.label, flasherReleaseDetail(release), selected: release.version == current.selected) {
                    command(FLASHER_SELECT, [release.version])
                }
            }
            if current.releasesState == "failed" {
                Text(current.releasesError).foregroundStyle(theme.aircast.warning)
            }
            if flasherShowsChannels(FLASHER_DEBUG_BUILD, current.channel) && !current.channels.isEmpty {
                Picker("Channel", selection: Binding(get: { current.channel }, set: { command(FLASHER_CHANNEL, [$0]) })) {
                    ForEach(current.channels, id: \.self) { channel in Text(channel.capitalizedFirst).tag(channel) }
                }
                .pickerStyle(.segmented)
            }
        } header: {
            SectionHeader(text: "Aircast OS version")
        }
        Section {
            Picker("Signing in", selection: Binding(get: { fields.sshMode }, set: { mode in
                edit(withChanges(fields) { $0.sshMode = mode }, "sshMode", .string(mode))
            })) {
                ForEach(FLASHER_SSH_MODES, id: \.first) { mode in Text(mode.second).tag(mode.first) }
            }
            .pickerStyle(.segmented)
            if fields.sshMode == "key-only" {
                FlasherField(label: "SSH public key", value: fields.authorizedKey, problem: problems["authorizedKey"], multiline: true) { typed in
                    edit(withChanges(fields) { $0.authorizedKey = typed }, "authorizedKey", .string(typed))
                }
                if !current.keyIdentity.isBlank {
                    Text(current.keyIdentity)
                        .font(.system(size: 12, design: .monospaced))
                        .textSelection(.enabled)
                }
            }
        } header: {
            SectionHeader(text: "Signing in to the drone")
        }
        Section {
            FlasherField(label: "Tailscale auth key", value: fields.authKey, help: "The drone joins your tailnet on first boot", secret: true) { typed in
                edit(withChanges(fields) { $0.authKey = typed }, "authKey", .string(typed))
            }
            if !fields.authKey.isBlank {
                FlasherField(label: "Headscale server", value: fields.controlServer, problem: problems["controlServer"], help: "Leave empty for Tailscale") { typed in
                    edit(withChanges(fields) { $0.controlServer = typed }, "controlServer", .string(typed))
                }
            }
        } header: {
            SectionHeader(text: "Remote access")
        }
        Section {
            FlasherField(label: "Country code", value: fields.country, help: "Sets which WiFi channels the drone may use") { typed in
                let code = String(typed.uppercased().prefix(2))
                edit(withChanges(fields) { $0.country = code }, "country", .string(code))
            }
        } header: {
            SectionHeader(text: "WiFi region")
        }
    }

    @ViewBuilder
    private func writeStep(_ current: FlasherState, _ fields: FlasherForm, _ meteredNote: String?) -> some View {
        if current.busy {
            Section {
                VStack(alignment: .leading, spacing: Space.s3) {
                    Text(flasherPhaseLabel(current.phase)).font(.headline)
                    ProgressView(value: min(max(current.percent / 100, 0), 1))
                    Text(["\(saturatingInt(current.percent))%", current.speedText, current.remainingText].filter { !$0.isBlank }.joined(separator: " · "))
                        .font(.subheadline)
                }
                .padding(.vertical, Space.s1)
            } footer: {
                Text("Keep the card reader plugged in until this finishes.")
            }
        } else if current.phase == "done" {
            heading("Card ready", "The card is written and checked. It's safe to unplug the reader.")
            Section {
                ForEach(Array(flasherNextSteps(fields.ssid.trimmed, current.hostname, fields.noWifi).enumerated()), id: \.offset) { index, line in
                    Text("\(index + 1). \(line)")
                }
            } header: {
                SectionHeader(text: "What happens next")
            }
        } else if current.phase == "failed" {
            heading("Writing failed", "The card is not usable as it is. Fix the problem below and write it again.")
            Section {
                Text(current.jobError)
                    .foregroundStyle(theme.aircast.warning)
                    .textSelection(.enabled)
            }
        } else if current.phase == "cancelled" {
            heading("Cancelled", "The card is incomplete. Write it again before putting it in the drone.")
        } else {
            heading("Check and write", "Everything on the card is replaced.")
            Section {
                ForEach(current.summary, id: \.label) { row in
                    summaryLine(row.label, row.value, warn: row.warn)
                }
                summaryLine("Card", current.cardText.ifBlank("Not connected"), warn: current.cardState != "ready")
            } footer: {
                if let meteredNote {
                    Text("\(meteredNote) It downloads when you write the card.").foregroundStyle(theme.aircast.warning)
                } else if current.downloadState != "ready" && !current.downloadText.isBlank {
                    Text(current.downloadText)
                }
            }
        }
    }

    private func summaryLine(_ label: String, _ value: String, warn: Bool) -> some View {
        LabeledContent(label) {
            HStack(spacing: Space.s2) {
                Text(value).foregroundStyle(warn ? theme.aircast.warning : Color.secondary)
                Button("Change") { change(label) }
            }
        }
    }

    private func change(_ row: String) {
        if FLASHER_MORE_ROWS.contains(row) { more = true }
        step = row == "Card" ? .Card : .Setup
    }
}

// A setup field as a native row: inline label and value, the problem (or the help) under it; a key spans its row.
private struct FlasherField: View {
    let label: String
    let value: String
    var problem: String? = nil
    var help: String? = nil
    var secret = false
    var multiline = false
    let onChange: (String) -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        if multiline {
            VStack(alignment: .leading, spacing: Space.s1) {
                Text(label).foregroundStyle(problem != nil ? theme.colors.error : Color.primary)
                TextField(label, text: Binding(get: { value }, set: onChange), axis: .vertical)
                    .lineLimit(3...8)
                    .labelsHidden()
                    .autocorrectionDisabled()
                    .font(.system(.body, design: .monospaced))
                if let note = problem ?? help {
                    Text(note).font(.footnote).foregroundStyle(problem != nil ? theme.colors.error : Color.secondary)
                }
            }
        } else {
            FieldRow(label: label, value: value, error: problem, hint: help ?? "", secure: secret, onChange: onChange)
        }
    }
}
#endif
