import SwiftUI

private let CAMERA_UNDO_WINDOW_MS = 6000
private let CAMERA_LIST_SETTLE_MS = 2000
private let CAMERA_CLASSIFY_SETTLE_MS = 300
private let ROW_PAD_HORIZONTAL = Space.s4
private let ROW_PAD_VERTICAL: CGFloat = 10
private let ROW_MIN_HEIGHT: CGFloat = 64
private let ROW_GAP: CGFloat = 14
private let ROW_LINE_GAP: CGFloat = 2
private let SHEET_PAD = Space.s6
private let SHEET_GAP = Space.s3

struct CameraDraft: Equatable {
    var stored: Int?
    var title: String
    var name: String
    var source: String
    var url: String
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
    let title: String
    let active: Bool
}

private func undoRemoval(_ gone: RemovedCamera, _ storedCount: Int) -> String? {
    let restored = CameraCommands.add(gone.name, source: gone.source, url: gone.url)
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
            needsUrl: !kinds.contains { $0.raw == camera.source && !$0.needsUrl },
            picked: camera.source,
            kept: camera.problem == nil ? camera.url : nil
        )
    }
}

private func draftGuess(_ draft: CameraDraft, _ guess: CameraGuess?) -> CameraGuess? {
    keptGuess(guess, draft.source, draft.url, draft.kept)
}

enum CameraSave: Equatable {
    case Add(name: String, source: String, url: String)
    case Update(slot: Int, name: String, source: String, url: String)
    case Refused(problem: String)
}

func cameraSave(_ draft: CameraDraft, _ shown: CameraGuess?, _ classify: (String) -> CameraGuess?) -> CameraSave {
    if let stored = draft.stored, !draft.needsUrl {
        return .Update(slot: stored, name: draft.name, source: draft.source, url: draft.url)
    }
    return savedAs(draft, shown ?? draftGuess(draft, classify(draft.url)))
}

private func savedAs(_ draft: CameraDraft, _ guess: CameraGuess?) -> CameraSave {
    guard let stored = draft.stored else { return .Add(name: draft.name, source: chosenKind(guess, draft.picked, ""), url: draft.url) }
    if let guess, guess.kind == nil, let problem = guess.problem { return .Refused(problem: problem) }
    return .Update(slot: stored, name: draft.name, source: chosenKind(guess, draft.picked, draft.source), url: draft.url)
}

private func written(_ save: CameraSave) -> String? {
    switch save {
    case .Add(let name, let source, let url): CameraCommands.add(name, source: source, url: url)
    case .Update(let slot, let name, let source, let url): CameraCommands.update(slot, name: name, source: source, url: url)
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
        let storedNow = reading?.stored.map { StoredCamera(name: $0.name, source: $0.source, url: $0.url) }
        let busy = pending || (settlingFrom != nil && settlingFrom == storedNow)
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                HStack {
                    Spacer()
                    Button { draft = CameraDraft(stored: nil, title: "", name: "", source: "", url: "") } label: {
                        Image(.add).font(.system(size: 20)).frame(width: 48, height: 48).contentShape(Rectangle())
                    }
                    .disabled(!editable || busy)
                    .accessibilityLabel("Add a video source")
                }
                .padding(.trailing, Space.s1)
                .padding(.top, Space.s2)
                if let reading, !reading.readable { ErrorLine(text: reading.reason) }
                if let notice { ErrorLine(text: notice) }
                if reading != nil && cameras.isEmpty {
                    FootNote(text: "No video sources yet. Cameras on the drone show up here by themselves. Tap + to add a stream by its address.")
                }
                ForEach(cameras) { camera in
                    CameraRow(camera: camera, onEdit: editDraft(camera, kinds).flatMap { opened in editable ? { draft = opened } : nil })
                    Divider()
                }
                if let gone = removed {
                    HStack {
                        Text("Removed \(gone.title).").font(.bodyMedium).frame(maxWidth: .infinity, alignment: .leading)
                        Button("Undo") {
                            removed = nil
                            let storedCount = reading?.stored.count ?? 0
                            change(storedNow, { undoRemoval(gone, storedCount) }) { notice = $0 }
                        }
                        .buttonStyle(.text)
                        .disabled(busy)
                    }
                    .padding(.horizontal, ROW_PAD_HORIZONTAL)
                    .padding(.vertical, Space.s1)
                }
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
                onPick: { kind in change(storedNow, { CameraCommands.add("", source: kind.raw, url: "") }, closed) },
                onRemove: current.stored.flatMap { slot in
                    cameras.first { $0.stored == slot }.map { entry -> () -> Void in
                        {
                            change(storedNow, { CameraCommands.remove(slot) }) { refusal in
                                notice = refusal
                                if refusal == nil {
                                    removed = RemovedCamera(slot: slot, name: entry.name, source: entry.source, url: entry.url, title: entry.title, active: entry.active)
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

private struct ErrorLine: View {
    let text: String
    @Environment(\.theme) private var theme

    var body: some View {
        Text(text)
            .font(.bodySmall)
            .foregroundStyle(theme.colors.error)
            .padding(.horizontal, ROW_PAD_HORIZONTAL)
            .padding(.vertical, Space.s1)
    }
}

private struct CameraRow: View {
    let camera: CameraEntry
    let onEdit: (() -> Void)?
    @Environment(\.theme) private var theme

    var body: some View {
        let row = HStack(spacing: ROW_GAP) {
            CameraStatusDot(status: camera.status)
            VStack(alignment: .leading, spacing: ROW_LINE_GAP) {
                Text(camera.title).font(.bodyLarge).foregroundStyle(theme.colors.onSurface).lineLimit(1)
                Text(cameraDetail(camera)).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant).lineLimit(1)
                if let problem = camera.problem {
                    Text(problem).font(.bodySmall).foregroundStyle(theme.colors.error)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            if camera.active {
                Text("On screen").font(.labelMedium).foregroundStyle(theme.colors.onSurfaceVariant)
            }
            if onEdit != nil {
                Image(.chevronRight).foregroundStyle(theme.colors.onSurfaceVariant)
            }
        }
        .padding(.horizontal, ROW_PAD_HORIZONTAL)
        .padding(.vertical, ROW_PAD_VERTICAL)
        .frame(minHeight: ROW_MIN_HEIGHT)
        .contentShape(Rectangle())
        if let onEdit {
            Button(action: onEdit) { row }.buttonStyle(.plain)
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
        let canSave = !busy && (!draft.url.isBlank || !draft.needsUrl)
        let save = { if canSave { onSave(guess) } }
        AircastSheet(onDismissRequest: onDismiss, skipPartiallyExpanded: true) {
            ScrollView {
                VStack(alignment: .leading, spacing: SHEET_GAP) {
                    Text(draft.stored == nil ? "Add video source" : draft.title).font(.titleMedium)
                    if draft.stored != nil { nameField(save) }
                    if draft.needsUrl {
                        address(guess, problem, save)
                    } else {
                        Text(kindLabel(draft.source)).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
                        if let refusal = draft.refusal {
                            Text(refusal).font(.bodySmall).foregroundStyle(theme.colors.error)
                        }
                    }
                    if draft.stored == nil { nameField(save) }
                    HStack {
                        if let onRemove {
                            Button("Remove", action: onRemove).foregroundStyle(theme.colors.error).buttonStyle(.text).disabled(busy)
                        }
                        Spacer()
                        Button("Cancel", action: onDismiss).buttonStyle(.text)
                        Button("Save", action: save).buttonStyle(.filled).disabled(!canSave)
                    }
                    if !others.isEmpty {
                        Text("Other sources")
                            .font(.labelLarge)
                            .foregroundStyle(theme.colors.onSurfaceVariant)
                            .padding(.top, Space.s2)
                        VStack(alignment: .leading, spacing: 0) {
                            ForEach(others, id: \.raw) { kind in
                                Button { onPick(kind) } label: {
                                    Text(otherSourceLabel(kind))
                                        .font(.bodyLarge)
                                        .foregroundStyle(theme.colors.onSurface)
                                        .frame(maxWidth: .infinity, alignment: .leading)
                                        .padding(.vertical, Space.s3)
                                        .contentShape(Rectangle())
                                }
                                .buttonStyle(.plain)
                                .disabled(busy)
                            }
                        }
                    }
                }
                .padding(.horizontal, SHEET_PAD)
                .padding(.bottom, SHEET_PAD)
            }
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
            field("Name") {
                TextField("Name", text: Binding(get: { draft.name }, set: { typed in
                    onChange(withChanges(draft) { $0.name = typed; $0.refusal = nil })
                }))
                .textInputAutocapitalization(.words)
                .submitLabel(.done)
                .onSubmit(save)
            }
        } else {
            Button("Add a name") { naming = true }.buttonStyle(.text)
        }
    }

    @ViewBuilder private func address(_ guess: CameraGuess?, _ problem: String?, _ save: @escaping () -> Void) -> some View {
        let supporting = problem ?? (draft.url.isBlank ? nil : guessText(guess))
        field("Address") {
            TextField("Address", text: Binding(get: { draft.url }, set: { typed in
                onChange(withChanges(draft) { $0.url = typed; $0.refusal = nil })
            }), prompt: Text(hint))
            .keyboardType(.URL)
            .textInputAutocapitalization(.never)
            .autocorrectionDisabled()
            .submitLabel(.done)
            .onSubmit(save)
        }
        if let supporting, !supporting.isBlank {
            Text(supporting).font(.bodySmall).foregroundStyle(problem != nil ? theme.colors.error : theme.colors.onSurfaceVariant)
        }
        if let ambiguous = guess, ambiguous.ambiguous {
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: Space.s2) {
                    ForEach(ambiguous.choices, id: \.self) { choice in
                        CameraChip(label: kindLabel(choice), selected: choice == chosenKind(ambiguous, draft.picked, "")) {
                            onChange(withChanges(draft) { $0.picked = choice; $0.refusal = nil })
                        }
                    }
                }
            }
        }
    }

    private func field<Field: View>(_ label: String, @ViewBuilder _ input: () -> Field) -> some View {
        VStack(alignment: .leading, spacing: Space.s1) {
            Text(label).font(.labelMedium).foregroundStyle(theme.colors.onSurfaceVariant)
            input().textFieldStyle(.roundedBorder)
        }
    }
}
