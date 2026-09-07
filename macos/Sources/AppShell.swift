import AppKit
import QGCBridgeC
import QGCEntry

enum AppShell {
    static func run(_ argc: Int32, _ argv: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>) -> Int32 {
        NativeDebug.install()
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
