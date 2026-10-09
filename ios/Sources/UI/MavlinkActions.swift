import SwiftUI

let MAVLINK_ACTIONS_VIEW = "view.mavlinkActions"
let MAVLINK_ACTIONS_SEND = "mavlinkActions.send"
let MAVLINK_ACTIONS_GROUP = "mavlinkActionsSettings"
let NO_ACTIONS_FILE = "<None>"

struct MavlinkActionEntry: Equatable {
    var label: String
    var description: String
}

struct MavlinkActions: Equatable {
    var files: [String]
    var flyViewFile: String
    var joystickFile: String
    var flyViewPath: String
    var joystickPath: String
    var actions: [MavlinkActionEntry]
    var folderNote: String = ""
}

func mavlinkActions(_ view: JSON?) -> MavlinkActions? {
    guard let view, let files = view["files"].arrayOrNil else { return nil }
    return MavlinkActions(
        files: files.map(\.string),
        flyViewFile: view["flyViewFile"].string,
        joystickFile: view["joystickFile"].string,
        flyViewPath: view["flyViewPath"].string,
        joystickPath: view["joystickPath"].string,
        actions: view["actions"].objects.map { MavlinkActionEntry(label: $0["label"].string, description: $0["description"].string) },
        folderNote: view["folderNote"].string
    )
}

func chosenFile(_ option: String) -> String { option == NO_ACTIONS_FILE ? "" : option }

struct MavlinkActionsSection: View {
    let onWrite: () -> Void
    @State private var revision = 0
    @State private var read: MavlinkActions?
    @State private var scope = ViewScope()
    @Environment(\.theme) private var theme

    var body: some View {
        ZStack(alignment: .topLeading) {
            if let actions = read {
                let options = [NO_ACTIONS_FILE] + actions.files
                VStack(alignment: .leading, spacing: 0) {
                    if !actions.folderNote.isBlank {
                        Text(actions.folderNote).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
                    }
                    FileChoice(label: "Fly view actions", current: actions.flyViewFile, options: options) { choose(actions.flyViewPath, $0) }
                    FileChoice(label: "Joystick actions", current: actions.joystickFile, options: options) { choose(actions.joystickPath, $0) }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, Space.s5)
                .padding(.vertical, Space.s2)
            }
        }
        .task(id: revision) {
            let fresh = await offMain { mavlinkActions(Qgc.get(MAVLINK_ACTIONS_VIEW)) }
            if !Task.isCancelled { read = fresh }
        }
        .onDisappear { scope.cancel() }
    }

    private func choose(_ path: String, _ option: String) {
        scope.launch {
            _ = await offMain { Qgc.set(path, chosenFile(option)) }
            guard !Task.isCancelled else { return }
            revision += 1
            onWrite()
        }
    }
}

private struct FileChoice: View {
    let label: String
    let current: String
    let options: [String]
    let onPick: (String) -> Void

    var body: some View {
        HStack {
            Text(label).frame(maxWidth: .infinity, alignment: .leading)
            Menu {
                ForEach(options, id: \.self) { option in
                    Button(option) { onPick(option) }
                }
            } label: {
                Text(current.ifBlank(NO_ACTIONS_FILE))
            }
            .buttonStyle(.bordered)
            .disabled(options.count <= 1)
        }
        .padding(.vertical, Space.s1)
    }
}

struct FlyViewMavlinkActions: View {
    let onSent: () -> Void
    @State private var read: MavlinkActions?
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(spacing: 0) {
            ForEach(Array((read?.actions ?? []).enumerated()), id: \.offset) { index, action in
                Button {
                    onSent()
                    offMain { Qgc.invoke(MAVLINK_ACTIONS_SEND, index) }
                } label: {
                    Text(action.label)
                        .font(.labelLarge)
                        .fontWeight(.bold)
                        .foregroundStyle(theme.colors.primary)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(.vertical, Space.s2)
                }
                .buttonStyle(.borderless)
            }
        }
        .task { read = await offMain { mavlinkActions(Qgc.get(MAVLINK_ACTIONS_VIEW)) } }
    }
}
