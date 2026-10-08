import SwiftUI

private let RESUME_MISSION_PATH = "planFly.missionController.resumeMission"

func resumeFailedIndex(_ view: JSON?) -> Int? {
    guard let view, view.has("resumeFailedIndex") else { return nil }
    return view["resumeFailedIndex"].int(0)
}

func resumeCleared(_ view: JSON?) -> Bool { view != nil && resumeFailedIndex(view) == nil }

struct ResumeFailedPrompt: View {
    let dismissed: Int?
    let onDismissed: (Int?) -> Void
    @QgcPath(GUIDED_ACTIONS) private var actions

    private var index: Int? { resumeFailedIndex(actions).flatMap { $0 != dismissed ? $0 : nil } }

    var body: some View {
        let index = index
        Color.clear
            .frame(width: 0, height: 0)
            .accessibilityHidden(true)
            .onChange(of: resumeCleared(actions), initial: true) { _, cleared in
                if cleared { onDismissed(nil) }
            }
            .alert(
                "Resume FAILED",
                isPresented: Binding(get: { index != nil }, set: { shown in if !shown, let index { onDismissed(index) } })
            ) {
                Button("Confirm") {
                    guard let index else { return }
                    onDismissed(index)
                    offMain { Qgc.invoke(RESUME_MISSION_PATH, index) }
                }
                Button("Cancel", role: .cancel) { if let index { onDismissed(index) } }
            } message: {
                Text("Upload of resume mission failed. Confirm to retry upload")
            }
    }
}
