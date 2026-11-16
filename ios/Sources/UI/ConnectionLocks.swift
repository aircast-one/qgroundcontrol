import SwiftUI

private let LOCK_TAG = "aircast:vehicle"

func vehicleConnected(_ view: JSON?) -> Bool { (view?["count"].int ?? 0) > 0 }

@MainActor
private enum VehicleLock {
    static var task = UIBackgroundTaskIdentifier.invalid

    static func hold(_ connected: Bool) {
        UIApplication.shared.isIdleTimerDisabled = connected
        guard connected else { return release() }
        guard task == .invalid else { return }
        task = UIApplication.shared.beginBackgroundTask(withName: LOCK_TAG) { MainActor.assumeIsolated { release() } }
    }

    static func release() {
        guard task != .invalid else { return }
        UIApplication.shared.endBackgroundTask(task)
        task = .invalid
    }
}

struct ConnectionLocks: View {
    @QgcPath(VEHICLES_VIEW) private var vehicles
    @Environment(\.scenePhase) private var phase

    private var keepAwake: Bool {
        let connected = vehicleConnected(vehicles)
        return connected
    }

    var body: some View {
        let awake = keepAwake
        Color.clear
            .invisibleAnchor()
            .onChange(of: awake, initial: true) { _, now in VehicleLock.hold(now) }
            .onChange(of: phase) { _, now in if now == .active { VehicleLock.hold(awake) } }
            .onDisappear { VehicleLock.hold(false) }
    }
}
