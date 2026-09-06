import AppKit
import QGCBridgeC
import QGCEntry

enum AppShell {
    static func run(_ argc: Int32, _ argv: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>) -> Int32 {
        let startCode = qgc_start(argc, argv)
        guard startCode == 0 else { return startCode }

        if CommandLine.arguments.contains("--native-window") {
            installMenuBar()
            QtHostWindow.shared.show()
        }

        let exitCode = qgc_run()
        qgc_shutdown()
        return exitCode
    }

    // Qt installs its own NSApplicationDelegate and depends on it for QFileOpenEvent
    // deep links, so the delegate stays Qt's until the last QML view is gone. The menu
    // bar is ours from here.
    private static func installMenuBar() {
        let appName = ProcessInfo.processInfo.processName

        let appMenu = NSMenu(title: appName)
        appMenu.addItem(withTitle: "About \(appName)", action: #selector(NSApplication.orderFrontStandardAboutPanel(_:)), keyEquivalent: "")
        appMenu.addItem(.separator())
        appMenu.addItem(withTitle: "Hide \(appName)", action: #selector(NSApplication.hide(_:)), keyEquivalent: "h")
        appMenu.addItem(withTitle: "Quit \(appName)", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")

        let windowMenu = NSMenu(title: "Window")
        windowMenu.addItem(withTitle: "Minimize", action: #selector(NSWindow.miniaturize(_:)), keyEquivalent: "m")
        windowMenu.addItem(withTitle: "Zoom", action: #selector(NSWindow.zoom(_:)), keyEquivalent: "")
        windowMenu.addItem(.separator())
        let telemetry = windowMenu.addItem(withTitle: "Native Telemetry", action: #selector(NativeWindow.showFromMenu), keyEquivalent: "t")
        telemetry.target = NativeWindow.shared

        let settings = appMenu.insertItem(withTitle: "Settings…", action: #selector(SettingsWindow.showFromMenu), keyEquivalent: ",", at: 1)
        settings.target = SettingsWindow.shared

        // The menu bar shows a submenu's own title, so an untitled carrier item still
        // renders correctly but is invisible to lookup by name. Title both.
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
