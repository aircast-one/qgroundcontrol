import Foundation
import QGCBridgeC

enum BridgeWatch {
    private static let lock = NSLock()
    private static var registry = BridgeWatchers()
    private static var handlers: [String: ([String: Any]) -> Void] = [:]
    private static var installed = false

    static func watch(_ client: String, _ paths: [String],
                      _ onChange: (([String: Any]) -> Void)? = nil) {
        lock.lock()
        registry.set(client, paths)
        handlers[client] = paths.isEmpty ? nil : onChange
        // Installed on the first watch and never withdrawn: qgc_core_set_event_handler starts a
        // thread on a OnceLock, so the core cannot take it back and a second install would only
        // replace a pointer with itself.
        let installing = !installed && !paths.isEmpty
        installed = installed || installing
        let csv = registry.csv(client)
        lock.unlock()

        if installing { qgc_bridge_set_event_handler(bridgeWatchEvent) }
        qgc_bridge_watch_client(client, csv)
    }

    static func stop(_ client: String) {
        watch(client, [])
    }

    static func deliver(_ path: String, _ json: String) {
        lock.lock()
        let called = registry.clients(watching: path).compactMap { handlers[$0] }
        lock.unlock()
        guard !called.isEmpty, let data = json.data(using: .utf8),
              let view = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any]
        else { return }

        // Two producers call this, and only one of them is Qt's thread. QGCBridgeCore's watcher
        // emits from the Qt thread, which drives this process's main thread; the core's own
        // qgc-core-pump thread calls the same handler for view.coreGuided, view.detections and
        // view.transports. So this hop is not a precaution against a thread that never happens --
        // remove it and those three write @Published from a background thread.
        guard Thread.isMainThread else {
            return DispatchQueue.main.async { called.forEach { $0(view) } }
        }
        called.forEach { $0(view) }
    }
}

private func bridgeWatchEvent(_ path: UnsafePointer<CChar>?, _ json: UnsafePointer<CChar>?) {
    guard let path, let json else { return }
    BridgeWatch.deliver(String(cString: path), String(cString: json))
}
