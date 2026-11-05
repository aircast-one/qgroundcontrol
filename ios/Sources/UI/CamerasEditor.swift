import SwiftUI

private let CAMERA_UNDO_WINDOW_MS = 6000
private let CAMERA_LIST_SETTLE_MS = 2000
private let CAMERA_CLASSIFY_SETTLE_MS = 300
private let GIMBAL_AIMED = "Aimed by a gimbal"
private let TILT_HELP = "For a camera fixed to the frame: 0 looks straight ahead, -90 straight down. Leave empty when a gimbal aims it. Used to place home and the plan on the video."
private let TILT_UNREADABLE = "Type the tilt as a number of degrees, like -30."

struct CameraDraft: Equatable {
    var stored: Int?
    var title: String
    var name: String
    var source: String
    var url: String
    var tilt: String = ""
    var needsUrl: Bool = true
    var picked: String? = nil
    var kept: String? = nil
    var refusal: String? = nil
}

private struct RemovedCamera: Equatable {
    let slot: Int
    let name: String
    let source: String
    let url: String
    let tilt: Double?
    let title: String
    let active: Bool
}

private func undoRemoval(_ gone: RemovedCamera, _ storedCount: Int) -> String? {
    let restored = CameraCommands.add(gone.name, source: gone.source, url: gone.url, tilt: gone.tilt)
        ?? (gone.slot < storedCount ? CameraCommands.move(storedCount, to: gone.slot) : nil)
    if restored == nil && gone.active { VideoCommands.setActiveSource(gone.slot) }
    return restored
}

private func editDraft(_ camera: CameraEntry, _ kinds: [CameraKind]) -> CameraDraft? {
    camera.stored.map { stored in
        CameraDraft(
            stored: stored,
            title: camera.title,
            name: camera.name,
            source: camera.source,
            url: camera.url,
            tilt: tiltText(camera.tilt),
            needsUrl: !kinds.contains { $0.raw == camera.source && !$0.needsUrl },
            picked: camera.source,
            kept: camera.problem == nil ? camera.url : nil
        )
    }
}

private func draftGuess(_ draft: CameraDraft, _ guess: CameraGuess?) -> CameraGuess? {
    keptGuess(guess, draft.source, draft.url, draft.kept)
}

func tiltText(_ tilt: Double?) -> String {
    tilt.map { $0.rounded() == $0 ? String(Int($0)) : String($0) } ?? ""
}

func typedTilt(_ text: String) -> Double? {
    Double(text.trimmingCharacters(in: .whitespaces).replacingOccurrences(of: ",", with: "."))
}

func tiltReadable(_ text: String) -> Bool {
    text.isBlank || typedTilt(text) != nil
}

enum CameraSave: Equatable {
    case Add(name: String, source: String, url: String, tilt: Double?)
    case Update(slot: Int, name: String, source: String, url: String, tilt: Double?)
    case Refused(problem: String)
}

func cameraSave(_ draft: CameraDraft, _ shown: CameraGuess?, _ classify: (String) -> CameraGuess?) -> CameraSave {
    if let stored = draft.stored, !draft.needsUrl {
        return .Update(slot: stored, name: draft.name, source: draft.source, url: draft.url, tilt: typedTilt(draft.tilt))
    }
    return savedAs(draft, shown ?? draftGuess(draft, classify(draft.url)))
}

private func savedAs(_ draft: CameraDraft, _ guess: CameraGuess?) -> CameraSave {
    let tilt = typedTilt(draft.tilt)
    guard let stored = draft.stored else { return .Add(name: draft.name, source: chosenKind(guess, draft.picked, ""), url: draft.url, tilt: tilt) }
    if let guess, guess.kind == nil, let problem = guess.problem { return .Refused(problem: problem) }
    return .Update(slot: stored, name: draft.name, source: chosenKind(guess, draft.picked, draft.source), url: draft.url, tilt: tilt)
}

private func written(_ save: CameraSave) -> String? {
    switch save {
    case .Add(let name, let source, let url, let tilt): CameraCommands.add(name, source: source, url: url, tilt: tilt)
    case .Update(let slot, let name, let source, let url, let tilt): CameraCommands.update(slot, name: name, source: source, url: url, tilt: tilt)
    case .Refused(let problem): problem
    }
}

private func saved(_ draft: CameraDraft, _ shown: CameraGuess?) -> String? {
    written(cameraSave(draft, shown) { address in cameraGuess(CameraCommands.classify(address)) })
}

func refusedDraft(_ current: CameraDraft?, _ refusal: String?) -> CameraDraft? {
    guard let refusal, var current else { return nil }
    current.refusal = refusal
    return current
}

private struct StoredCamera: Equatable {
    let name: String
    let source: String
    let url: String
    let tilt: Double?
}

struct CamerasEditor: View {
    @Environment(\.theme) private var theme
    @QgcPath(CAMERAS_VIEW) private var view
    @State private var draft: CameraDraft?
    @State private var notice: String?
    @State private var removed: RemovedCamera?
    @State private var pending = false
    @State private var settlingFrom: [StoredCamera]?

    var body: some View {
        let reading = camerasReading(view)
        let cameras = reading?.cameras ?? []
        let kinds = reading?.kinds ?? []
        let editable = reading?.readable == true
        let storedNow = reading?.stored.map { StoredCamera(name: $0.name, source: $0.source, url: $0.url, tilt: $0.tilt) }
        let busy = pending || (settlingFrom != nil && settlingFrom == storedNow)
        NativeForm {
            if let reading, !reading.readable { FormNote(text: reading.reason, color: theme.colors.error) }
            if let notice { FormNote(text: notice, color: theme.colors.error) }
            Section {
                if reading != nil && cameras.isEmpty {
                    Text("No video sources yet. Cameras on the drone show up here by themselves. Tap + to add a stream by its address.")
                        .foregroundStyle(Color.secondary)
                }
                ForEach(cameras) { camera in
                    CameraRow(camera: camera, onEdit: editDraft(camera, kinds).flatMap { opened in editable ? { draft = opened } : nil })
                }
                if let gone = removed {
                    LabeledContent("Removed \(gone.title).") {
                        Button("Undo") {
                            removed = nil
                            let storedCount = reading?.stored.count ?? 0
                            change(storedNow, { undoRemoval(gone, storedCount) }) { notice = $0 }
                        }
                        .disabled(busy)
                    }
                }
            }
        }
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button { draft = CameraDraft(stored: nil, title: "", name: "", source: "", url: "") } label: {
                    Label("Add a video source", systemImage: "plus")
                }
                .disabled(!editable || busy)
                .macTooltip("Add a video source")
            }
        }
        .background { sheet(reading, cameras, kinds, storedNow, busy) }
        .task(id: settlingFrom) {
            guard settlingFrom != nil else { return }
            try? await Task.sleep(for: .milliseconds(CAMERA_LIST_SETTLE_MS))
            if !Task.isCancelled { settlingFrom = nil }
        }
        .task(id: removed) {
            guard removed != nil else { return }
            try? await Task.sleep(for: .milliseconds(CAMERA_UNDO_WINDOW_MS))
            if !Task.isCancelled { removed = nil }
        }
    }

    @ViewBuilder private func sheet(_ reading: CamerasReading?, _ cameras: [CameraEntry], _ kinds: [CameraKind], _ storedNow: [StoredCamera]?, _ busy: Bool) -> some View {
        if let current = draft {
            let closed: @MainActor (String?) -> Void = { refusal in draft = refusedDraft(draft, refusal) }
            CameraSheet(
                draft: current,
                hint: kinds.first(where: \.needsUrl)?.hint ?? "",
                others: current.stored == nil ? otherSources(reading) : [],
                busy: busy,
                onChange: { next in if draft != nil { draft = next } },
                onDismiss: { draft = nil },
                onSave: { guess in change(storedNow, { saved(current, guess) }, closed) },
                onPick: { kind in change(storedNow, { CameraCommands.add(otherSourceName(kind), source: kind.raw, url: "", tilt: nil) }, closed) },
                onRemove: current.stored.flatMap { slot in
                    cameras.first { $0.stored == slot }.map { entry -> () -> Void in
                        {
                            change(storedNow, { CameraCommands.remove(slot) }) { refusal in
                                notice = refusal
                                if refusal == nil {
                                    removed = RemovedCamera(slot: slot, name: entry.name, source: entry.source, url: entry.url, tilt: entry.tilt, title: entry.title, active: entry.active)
                                }
                                draft = nil
                            }
                        }
                    }
                }
            )
        }
    }

    private func change(_ before: [StoredCamera]?, _ action: @escaping @Sendable () -> String?, _ after: @escaping @MainActor (String?) -> Void) {
        pending = true
        offMainInOrder {
            let refusal = action()
            onMain {
                after(refusal)
                settlingFrom = refusal == nil ? before : nil
                pending = false
            }
        }
    }
}

// A video source as a native row: its state, name and address, "On screen" for the one shown, and (for one stored
// here) the disclosure chevron that opens its editor.
private struct CameraRow: View {
    let camera: CameraEntry
    let onEdit: (() -> Void)?
    @Environment(\.theme) private var theme

    var body: some View {
        let row = HStack(spacing: Space.s3) {
            CameraStatusDot(status: camera.status)
            VStack(alignment: .leading, spacing: 2) {
                Text(camera.title).foregroundStyle(Color.primary).lineLimit(1)
                Text(cameraDetail(camera)).font(.subheadline).foregroundStyle(Color.secondary).lineLimit(1)
                if let problem = camera.problem {
                    Text(problem).font(.subheadline).foregroundStyle(theme.colors.error)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            if camera.active {
                Text("On screen").foregroundStyle(Color.secondary)
            }
            if onEdit != nil {
                Image(systemName: "chevron.right")
                    .font(.footnote.weight(.semibold))
                    .foregroundStyle(Color.secondary.opacity(0.6))
                    .accessibilityHidden(true)
            }
        }
        .contentShape(Rectangle())
        if let onEdit {
            Button(action: onEdit) { row }.listRowButton()
        } else {
            row
        }
    }
}

private struct Guessed: Equatable {
    let url: String
    let guess: CameraGuess?
}

private struct ClassifyKey: Equatable {
    let url: String
    let needsUrl: Bool
}

private struct CameraSheet: View {
    let draft: CameraDraft
    let hint: String
    let others: [CameraKind]
    let busy: Bool
    let onChange: (CameraDraft) -> Void
    let onDismiss: () -> Void
    let onSave: (CameraGuess?) -> Void
    let onPick: (CameraKind) -> Void
    let onRemove: (() -> Void)?
    @Environment(\.theme) private var theme
    @State private var guessed: Guessed?
    @State private var naming: Bool

    init(draft: CameraDraft, hint: String, others: [CameraKind], busy: Bool, onChange: @escaping (CameraDraft) -> Void, onDismiss: @escaping () -> Void, onSave: @escaping (CameraGuess?) -> Void, onPick: @escaping (CameraKind) -> Void, onRemove: (() -> Void)?) {
        self.draft = draft
        self.hint = hint
        self.others = others
        self.busy = busy
        self.onChange = onChange
        self.onDismiss = onDismiss
        self.onSave = onSave
        self.onPick = onPick
        self.onRemove = onRemove
        _naming = State(initialValue: draft.stored != nil)
    }

    var body: some View {
        let guess = draftGuess(draft, guessed.flatMap { $0.url == draft.url ? $0.guess : nil })
        let problem = draft.refusal ?? (draft.url.isBlank ? nil : guess?.problem)
        let canSave = !busy && (!draft.url.isBlank || !draft.needsUrl) && tiltReadable(draft.tilt)
        let save = { if canSave { onSave(guess) } }
        Color.clear
            .invisibleAnchor()
            .queuedSheet(isPresented: Binding(get: { true }, set: { shown in if !shown { onDismiss() } })) {
                NavigationStack {
                    NativeForm {
                        Section {
                            if draft.stored != nil { nameField(save) }
                            if draft.needsUrl {
                                address(guess, problem, save)
                            } else {
                                LabeledContent("Source", value: kindLabel(draft.source))
                                if let refusal = draft.refusal {
                                    Text(refusal).foregroundStyle(theme.colors.error)
                                }
                            }
                            if draft.stored == nil { nameField(save) }
                            tiltField(save)
                        }
                        if !others.isEmpty {
                            Section {
                                ForEach(others, id: \.raw) { kind in
                                    Button(otherSourceLabel(kind)) { onPick(kind) }.disabled(busy)
                                }
                            } header: {
                                SectionHeader(text: "Other sources")
                            }
                        }
                        if let onRemove {
                            Section {
                                Button("Remove", role: .destructive, action: onRemove).disabled(busy)
                            }
                        }
                    }
                    .navigationTitle(draft.stored == nil ? "Add video source" : draft.title)
                    .navigationBarTitleDisplayMode(.inline)
                    .pageCancelAction(action: onDismiss)
                    .pagePrimaryAction("Save", enabled: canSave, action: save)
                }
                .presentationDetents([.large])
                .nativeSheetFrame()
            }
        .task(id: ClassifyKey(url: draft.url, needsUrl: draft.needsUrl)) {
            guard !draft.url.isBlank, draft.needsUrl else { return }
            try? await Task.sleep(for: .milliseconds(CAMERA_CLASSIFY_SETTLE_MS))
            guard !Task.isCancelled else { return }
            let typed = draft.url
            let found = await offMain { cameraGuess(CameraCommands.classify(typed)) }
            guessed = Guessed(url: typed, guess: found)
        }
    }

    @ViewBuilder private func nameField(_ save: @escaping () -> Void) -> some View {
        if naming {
            FieldRow(
                label: "Name",
                value: draft.name,
                autocapitalize: true,
                onSubmit: save,
                onChange: { typed in onChange(withChanges(draft) { $0.name = typed; $0.refusal = nil }) }
            )
        } else {
            Button("Add a name") { naming = true }
        }
    }

    @ViewBuilder private func address(_ guess: CameraGuess?, _ problem: String?, _ save: @escaping () -> Void) -> some View {
        FieldRow(
            label: "Address",
            value: draft.url,
            prompt: hint,
            error: problem,
            hint: draft.url.isBlank ? "" : guessText(guess),
            keyboard: .URL,
            onSubmit: save,
            onChange: { typed in onChange(withChanges(draft) { $0.url = typed; $0.refusal = nil }) }
        )
        if let ambiguous = guess, ambiguous.ambiguous {
            Picker("Kind", selection: Binding(
                get: { chosenKind(ambiguous, draft.picked, "") },
                set: { choice in onChange(withChanges(draft) { $0.picked = choice; $0.refusal = nil }) }
            )) {
                ForEach(ambiguous.choices, id: \.self) { choice in Text(kindLabel(choice)).tag(choice) }
            }
        }
    }

    @ViewBuilder private func tiltField(_ save: @escaping () -> Void) -> some View {
        let readable = tiltReadable(draft.tilt)
        FieldRow(
            label: "Tilt (°)",
            value: draft.tilt,
            prompt: GIMBAL_AIMED,
            error: readable ? nil : TILT_UNREADABLE,
            hint: TILT_HELP,
            keyboard: .numbersAndPunctuation,
            onSubmit: save,
            onChange: { typed in onChange(withChanges(draft) { $0.tilt = typed; $0.refusal = nil }) }
        )
    }
}
