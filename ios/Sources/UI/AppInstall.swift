#if os(macOS)
import AppKit
import Foundation
import Network
import Observation

// The install step of an app update on the native macOS app, as the old macOS head did it: the core downloads and
// verifies the .dmg, Aircast opens it in Finder and quits, and the pilot drags the new copy onto Applications and
// chooses Replace. A running app cannot replace its own bundle, which is why it quits instead of waiting.

enum InstallFailure: Equatable {
    // The verified .dmg is no longer where the core left it.
    case Missing
    // Finder would not open the .dmg.
    case NotOpened

    var text: String {
        switch self {
        case .Missing: "The downloaded installer is gone. Get it from the release page."
        case .NotOpened: "macOS could not open the installer."
        }
    }
}

struct InstallOutcome: Equatable {
    let file: String
    let failure: InstallFailure
}

func installFailure(fileExists: Bool, opened: Bool) -> InstallFailure? {
    if !fileExists { return .Missing }
    return opened ? nil : .NotOpened
}

@MainActor
@Observable
final class MacInstaller {
    static let shared = MacInstaller()

    private(set) var outcome: InstallOutcome?

    func forget(keeping keep: (InstallOutcome) -> Bool) {
        if let held = outcome, !keep(held) { outcome = nil }
    }

    // Opens the .dmg and quits, so the copy in Applications can be replaced. On failure the app stays up and the
    // outcome says why, beside the update it belongs to.
    func start(_ file: String) {
        outcome = nil
        let exists = !file.isEmpty && FileManager.default.fileExists(atPath: file)
        let opened = exists && NSWorkspace.shared.open(URL(fileURLWithPath: file))
        if let failure = installFailure(fileExists: exists, opened: opened) {
            outcome = InstallOutcome(file: file, failure: failure)
            return
        }
        DispatchQueue.main.async { NSApp.terminate(nil) }
    }
}

// Whether the connection the update would come down is metered (Low Data Mode, or a path macOS marks expensive such as
// a phone hotspot) — what ConnectivityManager.isActiveNetworkMetered answers on Android.
enum UpdateNetwork {
    private static let monitor: NWPathMonitor = {
        let made = NWPathMonitor()
        made.start(queue: DispatchQueue(label: "one.aircast.update-network"))
        return made
    }()

    // The monitor learns the path a moment after it starts; touch it when an update view appears so the answer is
    // ready by the time the pilot presses Download.
    static func warm() { _ = monitor }

    static var metered: Bool {
        let path = monitor.currentPath
        return path.isExpensive || path.isConstrained
    }
}
#endif
