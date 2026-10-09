import SwiftUI

private let RESUME_MISSION_PATH = "planFly.missionController.resumeMission"

func resumeFailedIndex(_ view: JSON?) -> Int? {
    guard let view, view.has("resumeFailedIndex") else { return nil }
    return view["resumeFailedIndex"].int(0)
}

func resumeCleared(_ view: JSON?) -> Bool { view != nil && resumeFailedIndex(view) == nil }

func resumePending(_ failed: Int?, _ dismissed: Int?) -> Int? { failed.flatMap { $0 != dismissed ? $0 : nil } }

struct ResumeFailedPrompt: View {
    @QgcPath(GUIDED_ACTIONS) private var actions

    var body: some View {
        let dialogs = AppDialogsState.shared
        Color.clear
            .invisibleAnchor()
            .onChange(of: resumeCleared(actions), initial: true) { _, cleared in
                if cleared { dialogs.resumeDismissed = nil }
            }
            .onChange(of: resumeFailedIndex(actions), initial: true) { _, failed in dialogs.resumeFailed = failed }
            .onDisappear { dialogs.resumeFailed = nil }
    }
}

struct ResumeFailedDialog: View {
    let index: Int
    let onDismiss: () -> Void

    var body: some View {
        Color.clear
            .invisibleAnchor()
            .alert("Resume FAILED", isPresented: .constant(true)) {
                Button("Confirm") {
                    onDismiss()
                    let index = index
                    offMain { Qgc.invoke(RESUME_MISSION_PATH, index) }
                }
                Button("Cancel", role: .cancel, action: onDismiss)
            } message: {
                Text("Upload of resume mission failed. Confirm to retry upload")
            }
    }
}
