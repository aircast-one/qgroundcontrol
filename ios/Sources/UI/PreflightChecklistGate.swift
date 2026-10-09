import SwiftUI

private let CHECKLIST_CLOSE_DELAY_MS = 1000

@Observable
final class PreflightChecklistState {
    var ticked: Set<String> = []
    var shown = false
    var popupShownFor: Int?
    var stateSent: Bool?
    var vehicleId: Int?

    func open() {
        shown = true
    }
}

struct PreflightChecklistReset: View {
    let checklist: PreflightChecklistState
    let available: Bool

    var body: some View {
        Color.clear
            .frame(width: 0, height: 0)
            .accessibilityHidden(true)
            .onChange(of: available, initial: true) { _, now in
                guard !now else { return }
                checklist.ticked = []
                checklist.popupShownFor = nil
                checklist.vehicleId = nil
            }
    }
}

private struct SentKey: Equatable {
    let passed: Bool
    let vehicleId: Int?
}

private struct PopupKey: Equatable {
    let vehicleId: Int?
    let deciding: Bool
}

struct PreflightChecklist: View {
    let checklist: PreflightChecklistState
    let deciding: Bool
    @QgcBool(settingControl("settings.appSettings.enforceChecklist")) private var enforceChecklist
    @QgcPath(PREFLIGHT) private var preflightJson
    @QgcPath(VEHICLES_VIEW) private var vehiclesJson
    @Environment(\.theme) private var theme

    var body: some View {
        let offered = preflightOffered(preflightJson)
        let checks = preflight(preflightJson)
        let vehicleId = activeVehicleId(vehiclesJson)
        let passed = checklistIsComplete(checks, checklist.ticked)
        Color.clear
            .frame(width: 0, height: 0)
            .accessibilityHidden(true)
            .onChange(of: vehicleId, initial: true) { _, now in
                guard checklist.vehicleId != now else { return }
                checklist.vehicleId = now
                checklist.ticked = []
                checklist.stateSent = nil
            }
            .task(id: passed) {
                guard passed, checklist.shown,
                      (try? await Task.sleep(for: .milliseconds(CHECKLIST_CLOSE_DELAY_MS))) != nil else { return }
                checklist.shown = false
            }
            .task(id: SentKey(passed: passed, vehicleId: vehicleId)) {
                guard vehicleId != nil, checklist.stateSent != passed else { return }
                if checklist.stateSent == nil && !passed {
                    checklist.stateSent = false
                    return
                }
                checklist.stateSent = passed
                let state = checklistStateValue(passed)
                _ = await offMain { VehicleCommands.setChecklistState(state) }
            }
            .task(id: PopupKey(vehicleId: vehicleId, deciding: deciding)) {
                guard let id = vehicleId, checklist.popupShownFor != id, !deciding,
                      (try? await Task.sleep(for: .milliseconds(CHECKLIST_POPUP_DELAY_MS))) != nil else { return }
                let ticked = checklist.ticked
                let complete = await offMain { checklistIsComplete(preflight(Qgc.get(PREFLIGHT)), ticked) }
                guard !Task.isCancelled, checklistPopupIsDue(true, offered, enforceChecklist, complete, deciding: deciding) else { return }
                checklist.popupShownFor = id
                checklist.shown = true
            }
            .queuedSheet(isPresented: Binding(get: { checklist.shown }, set: { checklist.shown = $0 })) {
                NavigationStack {
                    PreflightScreen(ticked: checklist.ticked, onTicked: { checklist.ticked = $0 })
                        .navigationTitle("Pre-flight checklist")
                        .navigationBarTitleDisplayMode(.inline)
                        .toolbar {
                            ToolbarItem(placement: .cancellationAction) {
                                Button("Close") { checklist.shown = false }
                            }
                        }
                }
                .presentationDetents([.medium, .large])
                .presentationDragIndicator(.visible)
                .presentationBackground(theme.colors.surfaceContainerHigh)
            }
    }
}
