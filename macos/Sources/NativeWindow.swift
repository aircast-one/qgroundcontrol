import AppKit
import SwiftUI

final class NativeWindow: NSObject, NSWindowDelegate {
    static let shared = NativeWindow()

    private let telemetry = Telemetry()
    private var window: NSWindow?

    @objc func showFromMenu() {
        show()
    }

    func show() {
        if let window {
            window.makeKeyAndOrderFront(nil)
            return
        }

        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 400, height: 600),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false)
        window.title = "Native Telemetry"
        window.subtitle = "Phase 0 spike"
        window.isReleasedWhenClosed = false
        window.delegate = self
        window.contentView = NSHostingView(rootView: TelemetryView(telemetry: telemetry))
        window.center()
        window.makeKeyAndOrderFront(nil)

        self.window = window
        telemetry.start()
    }

    func windowWillClose(_ notification: Notification) {
        telemetry.stop()
        window = nil
    }
}

@_cdecl("qgc_native_window_show")
public func qgcNativeWindowShow() {
    NativeWindow.shared.show()
}
