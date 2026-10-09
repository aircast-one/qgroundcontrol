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

private func topmostController() -> UIViewController? {
    let windows = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.flatMap(\.windows)
    let root = (windows.first(where: \.isKeyWindow) ?? windows.first)?.rootViewController
    return root.flatMap { Array(sequence(first: $0, next: \.presentedViewController)).last { !$0.isBeingDismissed } }
}

@MainActor
private final class TopmostPrompter {
    private var wanted: CorePrompt?
    private var answered: CorePrompt?
    private var shown: CorePromptAlert?

    func show(_ prompt: CorePrompt?) {
        wanted = prompt
        if prompt != answered { answered = nil }
        guard prompt != shown?.prompt else { return }
        shown.map { alert in
            alert.onGone = {}
            alert.dismiss(animated: true)
        }
        shown = nil
        guard let prompt, prompt != answered, let top = topmostController() else { return }
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
        shown = alert
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
        .onAppear { offMainInOrder { PlanCommands.setUndoTracking(true) } }
        .onDisappear {
            offMainInOrder { PlanCommands.setUndoTracking(false) }
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
            Color.clear.frame(width: 0, height: 0).alert(
                pending.map { confirmCopy($0).title } ?? "",
                isPresented: Binding(get: { pending != nil }, set: { if !$0 { cancelPending() } }),
                presenting: pending
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
            Color.clear.frame(width: 0, height: 0).confirmationDialog(
                "Import as which pattern?",
                isPresented: Binding(get: { !files.patternChoice.options().isEmpty }, set: { if !$0 { files.patternChoice.cancel() } }),
                titleVisibility: .visible
            ) {
                ForEach(files.patternChoice.options(), id: \.self) { name in
                    Button(name) { files.patternChoice.pick(name) }
                }
                Button("Cancel", role: .cancel) { files.patternChoice.cancel() }
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
                Menu {
                    Button { offMainInOrder { PlanCommands.undo() } } label: { Label { Text("Undo") } icon: { Image(.undo) } }
                        .disabled(!history.canUndo)
                    Button { offMainInOrder { PlanCommands.redo() } } label: { Label { Text("Redo") } icon: { Image(.redo) } }
                        .disabled(!history.canRedo)
                    Divider()
                    Button { if dirty { pending = .Open } else { files.open() } } label: { Label { Text("Open plan…") } icon: { Image(.description) } }
                        .disabled(!can.open)
                    Button { files.save() } label: { Label { Text("Save") } icon: { Image(.download) } }
                        .disabled(!can.save)
                    Button { files.saveAs() } label: { Label { Text("Save as…") } icon: { Image(.edit) } }
                        .disabled(!can.save)
                    Button { files.exportKml() } label: { Label { Text("Export KML…") } icon: { Image(.send) } }
                        .disabled(!can.exportKml)
                    Button { files.importBoundary() } label: { Label { Text("Import boundary…") } icon: { Image(.map) } }
                        .disabled(!can.open)
                    Divider()
                    Button { showDefaults = true } label: { Label { Text("Defaults…") } icon: { Image(.tune) } }
                    Button { showTransform = true } label: { Label { Text("Transform…") } icon: { Image(.straighten) } }
                        .disabled(!containsItems)
                    Divider()
                    Button { if containsItems { pending = .NewPlan } else { files.newPlan() } } label: { Label { Text("New plan…") } icon: { Image(.add) } }
                        .disabled(!can.newPlan)
                    Button { if dirty { pending = .Download } else { files.download() } } label: { Label { Text("Load from vehicle") } icon: { Image(.upload) } }
                        .disabled(!can.download)
                    Button(role: .destructive) { pending = .ClearMission } label: { Label { Text("Clear mission") } icon: { Image(.delete) } }
                        .disabled(!can.clearFromVehicle)
                } label: {
                    Image(.moreVert).frame(width: 48, height: 48)
                }
                .accessibilityLabel("Plan menu")
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
