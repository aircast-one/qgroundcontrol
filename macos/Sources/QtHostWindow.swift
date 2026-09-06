import AppKit
import QGCEntry

final class QtHostView: NSView {
    private var embedded = false

    override var isFlipped: Bool { true }
    override var acceptsFirstResponder: Bool { true }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        guard window != nil, !embedded else { return }
        embedded = true
        qgc_embed_main_window(Unmanaged.passUnretained(self).toOpaque())
        pushSize()
    }

    override func setFrameSize(_ newSize: NSSize) {
        super.setFrameSize(newSize)
        pushSize()
    }

    private func pushSize() {
        guard embedded else { return }
        qgc_resize_main_window(Int32(bounds.width.rounded()), Int32(bounds.height.rounded()))
    }
}

final class QtHostWindow: NSObject, NSWindowDelegate {
    static let shared = QtHostWindow()

    private var window: NSWindow?

    func show() {
        if let window {
            window.makeKeyAndOrderFront(nil)
            return
        }

        let frame = NSRect(x: 0, y: 0, width: 1400, height: 900)
        let window = NSWindow(
            contentRect: frame,
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false)
        window.title = "Aircast QGC"
        window.delegate = self
        window.isReleasedWhenClosed = false
        window.contentView = QtHostView(frame: frame)
        window.center()
        window.makeKeyAndOrderFront(nil)

        self.window = window
    }

    func windowWillClose(_ notification: Notification) {
        NSApp.terminate(nil)
    }
}
