import SwiftUI
#if os(iOS)
import UIKit
#elseif os(macOS)
import AppKit
#endif

#if os(iOS)
final class AppDelegate: NSObject, UIApplicationDelegate {
    func application(_ application: UIApplication,
                     didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil) -> Bool {
        QgcTileProtocol.install()
        CoreHost.start()
        return true
    }

    func applicationWillTerminate(_ application: UIApplication) {
        CoreHost.stop()
    }
}
#elseif os(macOS)
final class AppDelegate: NSObject, NSApplicationDelegate {
    // Will-finish runs before SwiftUI builds the first window, as did-finish-launching does on iOS, so the core is up
    // before any view reads it.
    func applicationWillFinishLaunching(_ notification: Notification) {
        QgcTileProtocol.install()
        CoreHost.start()
        if CoreHost.isDebugBuild { NativeDebug.install() }
    }

    func applicationWillTerminate(_ notification: Notification) {
        MainActor.assumeIsolated { CoreHost.stop() }
    }

    // One window holds the whole app, as the single scene does on iOS.
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }
}
#endif

@main
struct AircastApp: App {
    #if os(iOS)
    @UIApplicationDelegateAdaptor(AppDelegate.self) private var delegate
    #elseif os(macOS)
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var delegate
    #endif

    var body: some Scene {
        #if os(macOS)
        // The app drives one vehicle session, so it keeps to the one window; the menu bar replaces File > New Window.
        mainWindow
            .defaultSize(width: MAC_WINDOW_DEFAULT_WIDTH, height: MAC_WINDOW_DEFAULT_HEIGHT)
            .windowResizability(.contentMinSize)
            .commands { MacMenuCommands() }
        #else
        mainWindow
        #endif
    }

    private var windowContent: some View {
        #if os(macOS)
        // Never narrow enough for the Fly tab's controls to crowd, and never phone-shaped.
        return MainActivity().frame(minWidth: MAC_WINDOW_MIN_WIDTH, minHeight: MAC_WINDOW_MIN_HEIGHT)
        #else
        return MainActivity()
        #endif
    }

    private var mainWindow: some Scene {
        WindowGroup {
            windowContent
                .onOpenURL { url in
                    if url.isFileURL {
                        PlanInbox.shared.received = url
                    } else if !DebugUiReceiver.onReceive(url) {
                        CoreHost.handleDeepLink(url)
                    }
                }
        }
    }
}
