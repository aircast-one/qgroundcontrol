import AppKit
import QGCBridgeC
import QGCEntry

final class QuitTarget: NSObject {
    @objc func requestQuit() {
        qgc_request_quit()
    }
}

/// Owns the handshake between AppKit on the main thread and Qt's event loop on its own thread.
final class QtRuntime {
    private let started = DispatchSemaphore(value: 0)
    private let finished = DispatchSemaphore(value: 0)
    private var startCode: Int32 = 0
    private var exitCode: Int32 = 0
    private var stopsMainLoop = false

    func start(_ argc: Int32, _ argv: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>) -> Int32 {
        let thread = Thread { [self] in
            startCode = qgc_start(argc, argv)
            started.signal()
            guard startCode == 0 else {
                exitCode = startCode
                finish()
                return
            }
            exitCode = qgc_run()
            qgc_shutdown()
            finish()
        }
        thread.name = "qt"
        thread.stackSize = 8 << 20
        thread.start()
        started.wait()
        return startCode
    }

    /// Blocks until Qt's loop has ended and the core is shut down.
    func wait() -> Int32 {
        finished.wait()
        return exitCode
    }

    /// From here on, Qt finishing must also end AppKit's loop.
    func stopsMainLoopWhenDone() {
        stopsMainLoop = true
    }

    private func finish() {
        finished.signal()
        guard stopsMainLoop else { return }
        DispatchQueue.main.async {
            NSApp.stop(nil)
            // stop() is only read between events, so hand the loop one.
            let wake = NSEvent.otherEvent(with: .applicationDefined, location: .zero, modifierFlags: [],
                                          timestamp: 0, windowNumber: 0, context: nil, subtype: 0, data1: 0, data2: 0)
            if let wake {
                NSApp.postEvent(wake, atStart: true)
            }
        }
    }
}

final class AppDelegate: NSObject, NSApplicationDelegate {
    private let runtime: QtRuntime

    init(runtime: QtRuntime) {
        self.runtime = runtime
    }

    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        // Qt closes links and video on quit; let it finish before AppKit tears the process down.
        qgc_request_quit()
        _ = runtime.wait()
        return .terminateNow
    }

    func application(_ application: NSApplication, open urls: [URL]) {
        for url in urls {
            qgc_handle_deep_link(url.absoluteString)
        }
    }
}

enum AppShell {
    static let quitTarget = QuitTarget()
    private static var delegate: AppDelegate?

    static func run(_ argc: Int32, _ argv: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>) -> Int32 {
        NativeDebug.install()
        Speech.install()
        let nativeWindows = LaunchMode.drawsNativeWindows(
            arguments: CommandLine.arguments,
            bundleDeclaresNativeUI: LaunchMode.bundleDeclaresNativeUI(Bundle.main.infoDictionary))
        qgc_set_host_provides_ui(nativeWindows ? 1 : 0)

        // A GUI-linked core builds a QGuiApplication, which macOS requires on the main thread,
        // and Qt's loop is then also AppKit's. Only the headless core can hand the main thread over.
        guard qgc_core_headless() != 0 else {
            return runWithQtOnMainThread(argc, argv, nativeWindows: nativeWindows)
        }
        return runWithQtOnItsOwnThread(argc, argv, nativeWindows: nativeWindows)
    }

    private static func runWithQtOnMainThread(_ argc: Int32,
                                              _ argv: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
                                              nativeWindows: Bool) -> Int32 {
        let startCode = qgc_start(argc, argv)
        guard startCode == 0 else { return startCode }

        if nativeWindows {
            installMenuBar()
            FlyWindow.shared.show()
        }

        let exitCode = qgc_run()
        Speech.shutdown()
        qgc_shutdown()
        return exitCode
    }

    private static func runWithQtOnItsOwnThread(_ argc: Int32,
                                                _ argv: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
                                                nativeWindows: Bool) -> Int32 {
        let runtime = QtRuntime()
        let startCode = runtime.start(argc, argv)
        guard startCode == 0 else { return startCode }

        // Boot tests and --list-tests finish inside qgc_run and never want a window.
        guard nativeWindows, qgc_runs_event_loop() != 0 else {
            let code = runtime.wait()
            Speech.shutdown()
            return code
        }

        let app = NSApplication.shared
        app.setActivationPolicy(.regular)
        delegate = AppDelegate(runtime: runtime)
        app.delegate = delegate
        installMenuBar()
        FlyWindow.shared.show()
        app.activate(ignoringOtherApps: true)

        runtime.stopsMainLoopWhenDone()
        app.run()

        Speech.shutdown()
        return runtime.wait()
    }

    private static func installMenuBar() {
        let appName = ProcessInfo.processInfo.processName

        let appMenu = NSMenu(title: appName)
        appMenu.addItem(withTitle: "About \(appName)", action: #selector(NSApplication.orderFrontStandardAboutPanel(_:)), keyEquivalent: "")
        appMenu.addItem(.separator())
        appMenu.addItem(withTitle: "Hide \(appName)", action: #selector(NSApplication.hide(_:)), keyEquivalent: "h")
        let quit = appMenu.addItem(withTitle: "Quit \(appName)", action: #selector(QuitTarget.requestQuit), keyEquivalent: "q")
        quit.target = AppShell.quitTarget

        let windowMenu = NSMenu(title: "Window")
        windowMenu.addItem(withTitle: "Minimize", action: #selector(NSWindow.miniaturize(_:)), keyEquivalent: "m")
        windowMenu.addItem(withTitle: "Zoom", action: #selector(NSWindow.zoom(_:)), keyEquivalent: "")
        windowMenu.addItem(.separator())
        let plan = windowMenu.addItem(withTitle: "Plan", action: #selector(PlanWindow.showFromMenu), keyEquivalent: "p")
        plan.target = PlanWindow.shared

        let setup = windowMenu.addItem(withTitle: "Vehicle Setup", action: #selector(VehicleSetupWindow.showFromMenu), keyEquivalent: "u")
        setup.target = VehicleSetupWindow.shared

        let analyze = windowMenu.addItem(withTitle: "Analyze", action: #selector(AnalyzeWindow.showFromMenu), keyEquivalent: "a")
        analyze.target = AnalyzeWindow.shared

        let fly = windowMenu.addItem(withTitle: "Fly", action: #selector(FlyWindow.showFromMenu), keyEquivalent: "f")
        fly.target = FlyWindow.shared

        let settings = appMenu.insertItem(withTitle: "Settings…", action: #selector(SettingsWindow.showFromMenu), keyEquivalent: ",", at: 1)
        settings.target = SettingsWindow.shared

        let bar = NSMenu()
        for menu in [appMenu, windowMenu] {
            let item = NSMenuItem()
            item.title = menu.title
            item.submenu = menu
            bar.addItem(item)
        }

        NSApp.mainMenu = bar
        NSApp.windowsMenu = windowMenu
    }
}

@_cdecl("qgc_macos_main")
public func qgcMacosMain(_ argc: Int32, _ argv: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>) -> Int32 {
    AppShell.run(argc, argv)
}
