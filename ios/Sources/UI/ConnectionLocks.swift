import SwiftUI

private let LOCK_TAG = "aircast:vehicle"

func vehicleConnected(_ view: JSON?) -> Bool { (view?["count"].int ?? 0) > 0 }

#if os(iOS)
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
#elseif os(macOS)
// While a vehicle is connected the display stays on and App Nap keeps its hands off the link, as the idle timer and
// background task do on iOS.
@MainActor
private enum VehicleLock {
    static func hold(_ connected: Bool) {
        if connected {
            MacActivity.begin(LOCK_TAG, keepDisplayOn: true)
        } else {
            MacActivity.end(LOCK_TAG)
        }
    }
}
#endif

struct ConnectionLocks: View {
    @QgcPath(VEHICLES_VIEW) private var vehicles
    @Environment(\.scenePhase) private var phase

    var body: some View {
        let connected = vehicleConnected(vehicles)
        Color.clear
            .invisibleAnchor()
            .onChange(of: connected, initial: true) { _, now in VehicleLock.hold(now) }
            .onChange(of: phase) { _, now in if now == .active { VehicleLock.hold(connected) } }
            .onDisappear { VehicleLock.hold(false) }
    }
}
