import SwiftUI

let ROUTE_RECORDER_VIEW = "view.routeRecorder"
let ROUTE_START = "routeRecorder.start"
let ROUTE_STOP = "routeRecorder.stop"
let ROUTE_RENAME = "routeRecorder.rename"
let ROUTE_DISMISS = "routeRecorder.dismiss"

private let ROUTE_SUBJECT = "The route recorder"
private let BLINK_SECONDS = 0.6
private let BLINK_LOW = 0.2
private let DOT_SIZE: CGFloat = 10
private let NOTICE_WIDTH: CGFloat = 420
private let PILL_ALPHA = 0.85
private let NOTICE_ALPHA = 0.9

struct RouteRecorder: Equatable {
    let available: Bool
    let recording: Bool
    let text: String?
    let savedFile: String?
    let savedName: String?
    let message: String?
}

func routeRecorder(_ view: JSON?) -> RouteRecorder? {
    guard let view, view["class"].string == "RouteRecorder" else { return nil }
    return RouteRecorder(
        available: view["available"].bool,
        recording: view["recording"].bool,
        text: view["text"].string.nonBlank,
        savedFile: view["savedFile"].string.nonBlank,
        savedName: view["savedName"].string.nonBlank,
        message: view["message"].string.nonBlank
    )
}

struct Renamed: Equatable {
    let file: String
    let message: String?
}

func renamed(_ answer: JSON?, _ file: String) -> Renamed {
    Renamed(file: answer?["path"].string.nonBlank ?? file, message: refusal(answer, subject: ROUTE_SUBJECT))
}

func routeTile(_ recorder: RouteRecorder?, _ onToggle: @escaping (Bool) -> Void) -> MoreTile? {
    guard let shown = recorder, shown.available else { return nil }
    return MoreTile(
        label: shown.recording ? "Stop recording" : "Record route",
        icon: shown.recording ? .stopCircle : .route,
        enabled: true,
        warning: shown.recording,
        onClick: { onToggle(shown.recording) }
    )
}

func toggleRoute(_ recording: Bool) -> String? {
    refusal(Qgc.call(recording ? ROUTE_STOP : ROUTE_START), subject: ROUTE_SUBJECT)
}

func openRouteInPlan(_ file: String) -> String? {
    let loaded = planLoad(file)
    guard loaded == true else { return loadFailureMessage(loaded) ?? "" }
    Qgc.invoke(ROUTE_DISMISS)
    return nil
}

struct RouteRecordingPill: View {
    @Environment(FlyScreenState.self) private var flyScreen: FlyScreenState?
    @QgcPath(ROUTE_RECORDER_VIEW) private var json

    var body: some View {
        if let text = routeRecorder(json)?.text {
            RecordingPill(text: text) {
                let flyScreen = flyScreen
                Task {
                    if let refused = await offMain({ toggleRoute(true) }) { flyScreen?.refusal = refused }
                }
            }
        }
    }
}

struct RouteSavedNotice: View {
    let onOpenedInPlan: () -> Void
    @QgcPath(ROUTE_RECORDER_VIEW) private var json

    var body: some View {
        if let shown = routeRecorder(json), !shown.recording, let message = shown.message {
            SavedRouteCard(recorder: shown, message: message, onOpenedInPlan: onOpenedInPlan)
        }
    }
}

struct RecordingPill: View {
    let text: String
    let onStop: () -> Void
    @Environment(\.theme) private var theme
    @State private var dim = false

    var body: some View {
        HStack(spacing: Space.s2) {
            Circle()
                .fill(Color.red)
                .frame(width: DOT_SIZE, height: DOT_SIZE)
                .opacity(dim ? BLINK_LOW : 1)
            Text(text).font(.labelLarge).foregroundStyle(theme.colors.onSurface).layoutPriority(-1)
            Button("Stop", action: onStop).buttonStyle(.text).padding(.trailing, Space.s2)
        }
        .padding(.leading, Space.s3)
        .background(theme.colors.surface.opacity(PILL_ALPHA), in: Capsule())
        .onAppear {
            withAnimation(.easeInOut(duration: BLINK_SECONDS).repeatForever(autoreverses: true)) { dim = true }
        }
    }
}

private struct SavedRouteCard: View {
    let recorder: RouteRecorder
    let message: String
    let onOpenedInPlan: () -> Void
    @Environment(\.theme) private var theme
    @Environment(OpenPlanDocument.self) private var openPlan: OpenPlanDocument?
    @State private var typed = ""
    @State private var said: String?
    @FocusState private var focused: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s2) {
            Text(said ?? message).font(.titleSmall)
            if recorder.savedFile != nil {
                VStack(alignment: .leading, spacing: Space.s1) {
                    Text("Name").font(.labelMedium).foregroundStyle(theme.colors.onSurfaceVariant)
                    TextField("Name", text: $typed)
                        .textFieldStyle(.roundedBorder)
                        .submitLabel(.done)
                        .focused($focused)
                        .onSubmit { focused = false }
                }
            }
            HStack(spacing: Space.s2) {
                Spacer(minLength: 0)
                if let file = recorder.savedFile {
                    Button("Open in Plan") { open(file) }.buttonStyle(.text)
                }
                Button("Dismiss") { offMain { Qgc.invoke(ROUTE_DISMISS) } }.buttonStyle(.text)
            }
        }
        .padding(Space.s3)
        .frame(maxWidth: NOTICE_WIDTH, alignment: .leading)
        .background(theme.colors.surface.opacity(NOTICE_ALPHA), in: RoundedRectangle(cornerRadius: Corner.medium))
        .onChange(of: recorder.savedName, initial: true) { _, name in typed = name ?? "" }
        .onChange(of: recorder.savedFile) { said = nil }
        .onChange(of: focused) { was, now in if was && !now { rename() } }
    }

    private func rename() {
        let wanted = typed.trimmed
        guard let file = recorder.savedFile, !wanted.isEmpty, wanted != recorder.savedName else {
            typed = recorder.savedName ?? ""
            return
        }
        Task {
            let answer = await offMain { renamed(Qgc.call(ROUTE_RENAME, file, wanted), file) }
            said = answer.message
            if answer.message != nil { typed = recorder.savedName ?? "" }
        }
    }

    private func open(_ file: String) {
        let openPlan = openPlan
        Task {
            if let failed = await offMain({ openRouteInPlan(file) }) {
                said = failed.nonBlank
            } else {
                openPlan?.document = nil
                openPlan?.name = nil
                onOpenedInPlan()
            }
        }
    }
}
