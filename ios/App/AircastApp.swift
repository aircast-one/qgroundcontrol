import SwiftUI
import UIKit

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

@main
struct AircastApp: App {
    @UIApplicationDelegateAdaptor(AppDelegate.self) private var delegate

    var body: some Scene {
        WindowGroup {
            AircastRoot()
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
