import SwiftUI
import UIKit

private let NOTICE_MILLIS = 4000

let APPLY_DEFAULT_ALTITUDE = "core.plan.applyDefaultAltitude"
let DISMISS_ALTITUDE_PROMPT = "core.plan.dismissAltitudePrompt"

struct AltitudePrompt: Equatable {
    let title: String
    let text: String
}

let LOAD_VEHICLE_PLAN = "core.plan.loadVehiclePlan"
let KEEP_CURRENT_PLAN = "core.plan.keepCurrentPlan"

struct VehicleChangePrompt: Equatable {
    let title: String
    let text: String
    let loadText: String
    let keepText: String
}

func vehicleChangePrompt(_ view: JSON?) -> VehicleChangePrompt? {
    guard let prompt = view?["vehicleChangePrompt"], prompt.object != nil else { return nil }
    return VehicleChangePrompt(title: prompt["title"].string, text: prompt["text"].string, loadText: prompt["loadText"].string, keepText: prompt["keepText"].string)
}

func applyAltitudePrompt(_ view: JSON?) -> AltitudePrompt? {
    guard let prompt = view?["applyAltitudePrompt"], prompt.object != nil else { return nil }
    return AltitudePrompt(title: prompt["title"].string, text: prompt["text"].string)
}

struct CorePromptChoice: Equatable {
    let label: String
    let invoke: String
    var cancel = false
}

struct CorePrompt: Equatable {
    let title: String
    let text: String
    let choices: [CorePromptChoice]
}

func corePrompt(_ view: JSON?) -> CorePrompt? {
    let vehicle = vehicleChangePrompt(view).map { prompt in
        CorePrompt(title: sentenceCase(prompt.title), text: prompt.text, choices: [
            CorePromptChoice(label: sentenceCase(prompt.loadText), invoke: LOAD_VEHICLE_PLAN),
            CorePromptChoice(label: sentenceCase(prompt.keepText), invoke: KEEP_CURRENT_PLAN),
        ])
    }
    return vehicle ?? applyAltitudePrompt(view).map { prompt in
        CorePrompt(title: sentenceCase(prompt.title), text: prompt.text, choices: [
            CorePromptChoice(label: "Yes", invoke: APPLY_DEFAULT_ALTITUDE),
            CorePromptChoice(label: "No", invoke: DISMISS_ALTITUDE_PROMPT, cancel: true),
        ])
    }
}

private final class CorePromptAlert: UIAlertController {
    var prompt: CorePrompt?
    var onGone: () -> Void = {}

    override func viewDidDisappear(_ animated: Bool) {
        super.viewDidDisappear(animated)
        onGone()
    }
}

private func settledTopmostController() -> UIViewController? {
    let windows = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.flatMap(\.windows)
    let root = (windows.first(where: \.isKeyWindow) ?? windows.first)?.rootViewController
    let chain = root.map { Array(sequence(first: $0, next: \.presentedViewController)) } ?? []
    return chain.contains { $0.isBeingPresented || $0.isBeingDismissed } ? nil : chain.last
}

@MainActor
private final class TopmostPrompter {
    private var wanted: CorePrompt?
    private var answered: CorePrompt?
    private var shown: CorePromptAlert?
    private var retrying = false

    func show(_ prompt: CorePrompt?) {
        wanted = prompt
        if prompt != answered { answered = nil }
        guard prompt != shown?.prompt else { return }
        shown.map { alert in
            alert.onGone = {}
            alert.dismiss(animated: true)
        }
        shown = nil
        guard let prompt, prompt != answered else { return }
        guard let top = settledTopmostController() else { return retry() }
        let alert = CorePromptAlert(title: prompt.title, message: prompt.text, preferredStyle: .alert)
        alert.prompt = prompt
        prompt.choices.forEach { choice in
            alert.addAction(UIAlertAction(title: choice.label, style: choice.cancel ? .cancel : .default) { [weak self] _ in
                self?.answer(prompt)
                offMain { Qgc.invoke(choice.invoke) }
            })
        }
        alert.onGone = { [weak self, weak alert] in self?.gone(alert) }
        top.present(alert, animated: true)
        guard alert.presentingViewController != nil else { return retry() }
        shown = alert
    }

    private func retry() {
        guard !retrying else { return }
        retrying = true
        Task { @MainActor in
            try? await Task.sleep(for: .milliseconds(SHEET_POLL_MS))
            retrying = false
            show(wanted)
        }
    }

    private func answer(_ prompt: CorePrompt) {
        answered = prompt
        shown?.onGone = {}
        shown = nil
    }

    private func gone(_ alert: CorePromptAlert?) {
        guard let alert, alert === shown else { return }
        shown = nil
        Task { @MainActor in
            try? await Task.sleep(for: .milliseconds(PROMPT_REPRESENT_MILLIS))
            show(wanted)
        }
    }
}

private let PROMPT_REPRESENT_MILLIS = 400
private let PLAN_MENU_ROW_HEIGHT: CGFloat = 44
private let PLAN_MENU_MIN_WIDTH: CGFloat = 220
private let PLAN_MENU_MAX_WIDTH: CGFloat = 280
private let DISABLED_MENU_ALPHA = 0.38

@MainActor
private enum PlanUndoTracking {
    private static var count = 0

    static func mounted(_ delta: Int) {
        count += delta
        let tracking = count > 0
        offMainInOrder { PlanCommands.setUndoTracking(tracking) }
    }
}

struct PlanTab: View {
    @Environment(\.theme) private var theme
    @QgcPath("view.plan") private var planStatus
    @State private var notice: String?
    @State private var pending: PlanConfirm?
    @State private var files = PlanFileActions()
    @State private var showDefaults = false
    @State private var showTransform = false
    @State private var undrawn: [String] = []
    @State private var centre: (Double, Double)?
    @State private var prompter = TopmostPrompter()
    @State private var incoming: URL?
    @State private var menuOpen = false

    var body: some View {
        let containsItems = planContainsItems(planStatus)
        let syncing = planIsSyncing(planStatus)
        PlanMapScreen(
            onCentre: { lat, lon in centre = (lat, lon) },
            itemEditor: { index, at, close, remove in
                AnyView(ItemEditor(index: index, at: at, mapCentre: centre, onDismiss: close, onRemove: remove))
            },
            header: { upload in AnyView(header(upload)) },
            fitKey: files.opened(),
            overlay: {
                AnyView(
                    PlanTemplates(planStatus: planStatus, centre: centre, onRefused: { notice = $0 })
                        .padding(.bottom, Space.s6)
                        .padding(.horizontal, Space.s4)
                        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottom)
                )
            },
            summaryHidden: planTemplates(planStatus)?.show == true
        )
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background { dialogs }
        .modifier(PlanFileDialogs(files: files, onResult: { notice = $0 }))
        .onAppear { PlanUndoTracking.mounted(1) }
        .onDisappear {
            PlanUndoTracking.mounted(-1)
            prompter.show(nil)
        }
        .onChange(of: corePrompt(planStatus), initial: true) { _, prompt in prompter.show(prompt) }
        .onChange(of: PlanInbox.shared.received, initial: true) { _, url in receive(url) }
        .task(id: [String(syncing), String(containsItems), files.documentName() ?? ""]) {
            undrawn = await offMain { undrawnItemNames(visualItems()) }
        }
        .task(id: notice) {
            guard notice != nil else { return }
            try? await Task.sleep(for: .milliseconds(NOTICE_MILLIS))
            if !Task.isCancelled { notice = nil }
        }
    }

    @ViewBuilder
    private var dialogs: some View {
        ZStack {
            if showDefaults {
                PlanDefaultsDialog(view: planStatus) { showDefaults = false }
            }
            if showTransform {
                PlanTransformDialog { showTransform = false }
            }
            Color.clear.frame(width: 0, height: 0).alertWhenPresenterFree(
                Binding(get: { pending }, set: { if $0 == nil { cancelPending() } }),
                title: { confirmCopy($0).title }
            ) { kind in
                let copy = confirmCopy(kind)
                let received = incoming
                Button(copy.confirm, role: copy.destructive ? .destructive : nil) {
                    cancelPending()
                    act(kind, received)
                }
                Button("Keep editing", role: .cancel) { cancelPending() }
            } message: { kind in
                Text(confirmCopy(kind).body)
            }
            Color.clear.frame(width: 0, height: 0).alertWhenPresenterFree(
                Binding(get: { Optional(files.patternChoice.options()).flatMap { $0.isEmpty ? nil : $0 } }, set: { if $0 == nil { files.patternChoice.cancel() } }),
                title: { _ in "Import as which pattern?" }
            ) { options in
                ForEach(options, id: \.self) { name in
                    Button(name) { files.patternChoice.pick(name) }
                }
                Button("Cancel", role: .cancel) { files.patternChoice.cancel() }
            } message: { _ in
                EmptyView()
            }
        }
    }

    private func cancelPending() {
        pending = nil
        incoming = nil
    }

    private func receive(_ url: URL?) {
        guard let url else { return }
        PlanInbox.shared.received = nil
        guard url.pathExtension.lowercased() != KML_EXTENSION else { return files.chooseBoundary([url]) }
        Task {
            if await offMain({ planIsDirty(Qgc.get("view.plan")) }) {
                incoming = url
                pending = .Open
            } else {
                files.openFrom(url)
            }
        }
    }

    private func act(_ kind: PlanConfirm, _ received: URL?) {
        switch kind {
        case .Open:
            if let received { files.openFrom(received) } else { files.open() }
        case .NewPlan: files.newPlan()
        case .ClearMission: files.clearMission()
        case .Download: files.download()
        }
    }

    @ViewBuilder
    private func header(_ upload: PlanUpload) -> some View {
        let history = planHistory(planStatus)
        let can = planActions(planStatus)
        let dirty = planIsDirty(planStatus)
        let containsItems = planContainsItems(planStatus)
        let title = planTitle(files.documentName())
        VStack(spacing: 0) {
            HStack(spacing: Space.s1) {
                VStack(alignment: .leading, spacing: 0) {
                    Text(title).font(.titleLarge).lineLimit(1).truncationMode(.tail)
                    let line = notice ?? planStatusText(planStatus)
                    if !line.isBlank && line != title {
                        Text(line)
                            .font(.bodySmall)
                            .foregroundStyle(theme.colors.onSurfaceVariant)
                            .lineLimit(notice == nil ? 1 : 3)
                            .truncationMode(.tail)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                if upload.shown { uploadButton(upload) }
                Button { menuOpen = true } label: {
                    Image(.moreVert).frame(width: 48, height: 48).contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .foregroundStyle(theme.colors.onSurface)
                .accessibilityLabel("Plan menu")
                .popover(isPresented: $menuOpen) {
                    ViewThatFits(in: .vertical) {
                        planMenu(history, can, dirty, containsItems)
                        ScrollView { planMenu(history, can, dirty, containsItems) }.scrollIndicatorsFlash(onAppear: true)
                    }
                    .frame(minWidth: PLAN_MENU_MIN_WIDTH, maxWidth: PLAN_MENU_MAX_WIDTH)
                    .presentationCompactAdaptation(.popover)
                    .presentationBackground(theme.colors.surfaceContainer)
                }
            }
            .frame(minHeight: 64)
            .padding(.leading, Space.s4)
            .padding(.trailing, Space.s1)
            if let warning = undrawnItemsWarning(undrawn) {
                Text(warning)
                    .font(.bodySmall)
                    .foregroundStyle(theme.colors.onErrorContainer)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, Space.s3)
                    .padding(.vertical, Space.s2)
                    .background(theme.colors.errorContainer)
            }
        }
    }

    private func planMenu(_ history: PlanHistory, _ can: PlanActions, _ dirty: Bool, _ containsItems: Bool) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            menuItem("Undo", .undo, enabled: history.canUndo) { offMainInOrder { PlanCommands.undo() } }
            menuItem("Redo", .redo, enabled: history.canRedo) { offMainInOrder { PlanCommands.redo() } }
            menuDivider
            menuItem("Open plan…", .description, enabled: can.open) { if dirty { pending = .Open } else { files.open() } }
            menuItem("Save", .download, enabled: can.save) { files.save() }
            menuItem("Save as…", .edit, enabled: can.save) { files.saveAs() }
            menuItem("Export KML…", .send, enabled: can.exportKml) { files.exportKml() }
            menuItem("Import boundary…", .map, enabled: can.open) { files.importBoundary() }
            menuDivider
            menuItem("Defaults…", .tune) { showDefaults = true }
            menuItem("Transform…", .straighten, enabled: containsItems) { showTransform = true }
            menuDivider
            menuItem("New plan…", .add, enabled: can.newPlan) { if containsItems { pending = .NewPlan } else { files.newPlan() } }
            menuItem("Load from vehicle", .upload, enabled: can.download) { if dirty { pending = .Download } else { files.download() } }
            menuItem("Clear mission", .delete, enabled: can.clearFromVehicle, destructive: true) { pending = .ClearMission }
        }
        .padding(.vertical, Space.s2)
    }

    private var menuDivider: some View {
        Divider().padding(.vertical, Space.s1)
    }

    private func menuItem(_ label: String, _ icon: Icon, enabled: Bool = true, destructive: Bool = false, _ action: @escaping () -> Void) -> some View {
        Button {
            menuOpen = false
            action()
        } label: {
            HStack(spacing: Space.s3) {
                Image(icon)
                    .frame(width: 24, height: 24)
                    .foregroundStyle(destructive ? theme.colors.error : theme.colors.onSurfaceVariant)
                Text(label)
                    .font(.labelLarge)
                    .foregroundStyle(destructive ? theme.colors.error : theme.colors.onSurface)
                    .lineLimit(1)
            }
            .padding(.horizontal, Space.s3)
            .frame(maxWidth: .infinity, minHeight: PLAN_MENU_ROW_HEIGHT, alignment: .leading)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .disabled(!enabled)
        .opacity(enabled ? 1 : DISABLED_MENU_ALPHA)
    }

    private func uploadButton(_ upload: PlanUpload) -> some View {
        let fill = upload.done ? theme.aircast.success : upload.emphasised ? theme.colors.primary : theme.colors.surfaceContainerHighest
        let ink = upload.done ? theme.aircast.onSuccess : upload.emphasised ? theme.colors.onPrimary : theme.colors.onSurface
        let progress = planSyncProgress(planStatus)
        return Button(action: upload.onClick) {
            HStack(spacing: Space.s2) {
                Image(upload.done ? .checkCircle : .upload).font(.system(size: 20))
                Text(upload.label).font(.labelLarge)
            }
            .padding(.leading, Space.s4)
            .padding(.trailing, Space.s5)
            .frame(height: 40)
            .foregroundStyle(ink)
            .background(alignment: .leading) {
                if planIsSyncing(planStatus) {
                    GeometryReader { geo in
                        Rectangle().fill(ink.opacity(0.28)).frame(width: geo.size.width * progress)
                    }
                }
            }
            .background(fill)
            .clipShape(Capsule())
        }
        .buttonStyle(.plain)
        .disabled(!upload.enabled)
        .opacity(upload.enabled ? 1 : 0.38)
    }
}
