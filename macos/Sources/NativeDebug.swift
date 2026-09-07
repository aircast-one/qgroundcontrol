import AppKit
import Foundation
import QGCNativeDebugC

enum NativeDebug {
    private static func raise(_ window: NSWindow) -> Bool {
        for _ in 0..<10 {
            NSApp.activate(ignoringOtherApps: true)
            window.makeKeyAndOrderFront(nil)
            RunLoop.current.run(until: Date().addingTimeInterval(0.15))
            if window.isKeyWindow { return true }
        }
        return false
    }

    private enum Resolved {
        case found(NSWindow)
        case failed(String)
    }

    private static func resolve(_ title: String) -> Resolved {
        let matches = NSApp.windows.filter { $0.title == title && $0.isVisible }
        switch matches.count {
        case 1: return .found(matches[0])
        case 0: return .failed("no visible window titled \(title)")
        default: return .failed("\(matches.count) visible windows are titled \(title)")
        }
    }

    static func windows() -> [String: Any] {
        let screenHeight = NSScreen.screens.first?.frame.height ?? 0
        let entries = NSApp.windows.enumerated().map { index, window -> [String: Any] in
            let frame = window.frame
            let content = window.contentView?.frame ?? .zero
            return [
                "index": index,
                "title": window.title,
                "number": window.windowNumber,
                "visible": window.isVisible,
                "key": window.isKeyWindow,
                "main": window.isMainWindow,
                "captureRect": [
                    "x": Int(frame.origin.x.rounded()),
                    "y": Int((screenHeight - frame.origin.y - frame.height).rounded()),
                    "width": Int(frame.width.rounded()),
                    "height": Int(frame.height.rounded()),
                ],
                "contentSize": ["width": Int(content.width.rounded()), "height": Int(content.height.rounded())],
                "hostsQt": window.contentView is QtHostView,
            ]
        }
        return ["windows": entries, "screenHeight": Int(screenHeight.rounded())]
    }

    static func click(window title: String, x: Double, y: Double) -> [String: Any] {
        let window: NSWindow
        switch NativeDebug.resolve(title) {
        case let .found(match): window = match
        case let .failed(reason): return ["ok": false, "error": reason]
        }
        guard let content = window.contentView else {
            return ["ok": false, "error": "window has no content view"]
        }
        if NativeProbe.screenIsLocked {
            return ["ok": false, "error": "screen is locked; synthesised clicks cannot reach a window that cannot become key — drive the UI through /native/probe instead"]
        }

        let frame = content.frame
        let point = NSPoint(x: frame.origin.x + x, y: frame.origin.y + (frame.height - y))
        guard content.bounds.contains(content.convert(point, from: nil)) else {
            return ["ok": false, "error": "point (\(x), \(y)) is outside the \(Int(frame.width))x\(Int(frame.height)) content view"]
        }

        for type in [NSEvent.EventType.leftMouseDown, .leftMouseUp] {
            guard let event = NSEvent.mouseEvent(
                with: type,
                location: point,
                modifierFlags: [],
                timestamp: ProcessInfo.processInfo.systemUptime,
                windowNumber: window.windowNumber,
                context: nil,
                eventNumber: 0,
                clickCount: 1,
                pressure: type == .leftMouseDown ? 1 : 0)
            else {
                return ["ok": false, "error": "could not synthesise \(type.rawValue)"]
            }
            window.sendEvent(event)
        }

        return ["ok": true, "windowPoint": ["x": Int(point.x), "y": Int(point.y)]]
    }

    static func type(window title: String, text: String) -> [String: Any] {
        let window: NSWindow
        switch NativeDebug.resolve(title) {
        case let .found(match): window = match
        case let .failed(reason): return ["ok": false, "error": reason]
        }
        if NativeProbe.screenIsLocked {
            return ["ok": false, "error": "screen is locked; typing needs a key window — drive the UI through /native/probe instead"]
        }
        guard NativeDebug.raise(window) else {
            return ["ok": false, "error": "window \(title) could not be raised; another app may hold focus"]
        }

        for character in text.unicodeScalars {
            var unit = UInt16(truncatingIfNeeded: character.value)
            for isDown in [true, false] {
                guard let event = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: isDown) else {
                    return ["ok": false, "error": "could not synthesise a key event"]
                }
                event.keyboardSetUnicodeString(stringLength: 1, unicodeString: &unit)
                event.post(tap: .cghidEventTap)
            }
        }
        RunLoop.current.run(until: Date().addingTimeInterval(0.2))
        return ["ok": true, "typed": text]
    }

    static func probe(id: String, action: String, args: [String: String]) -> [String: Any] {
        if id.isEmpty { return NativeProbe.tree() }
        if action.isEmpty { return NativeProbe.state(of: id) }
        return NativeProbe.invoke(id: id, action: action, args: args)
    }

    static func menu() -> [String: Any] {
        func describe(_ menu: NSMenu) -> [[String: Any]] {
            menu.items.map { item in
                var entry: [String: Any] = [
                    "title": item.submenu?.title.isEmpty == false ? item.submenu!.title : item.title,
                    "enabled": item.isEnabled,
                    "separator": item.isSeparatorItem,
                    "keyEquivalent": item.keyEquivalent,
                ]
                if let submenu = item.submenu {
                    entry["items"] = describe(submenu)
                }
                return entry
            }
        }
        guard let bar = NSApp.mainMenu else { return ["menu": []] }
        return ["menu": describe(bar)]
    }

    static func invokeMenu(path: String) -> [String: Any] {
        let parts = path.split(separator: "/").map(String.init)
        guard !parts.isEmpty, let bar = NSApp.mainMenu else {
            return ["ok": false, "error": "empty path or no menu bar"]
        }

        var menu: NSMenu? = bar
        var target: NSMenuItem?
        for part in parts {
            let match = menu?.items.first {
                $0.title == part || ($0.submenu?.title == part && !part.isEmpty)
            }
            guard let item = match else {
                return ["ok": false, "error": "no menu item \(part) in \(path)"]
            }
            target = item
            menu = item.submenu
        }

        guard let item = target else { return ["ok": false, "error": "unresolved \(path)"] }
        guard let action = item.action else { return ["ok": false, "error": "\(path) has no action"] }
        NSApp.sendAction(action, to: item.target, from: item)
        return ["ok": true, "invoked": path]
    }

    static func bridgeStats() -> [String: Any] {
        Bridge.Stats.snapshot
    }
}

private func encode(_ value: [String: Any]) -> UnsafeMutablePointer<CChar>? {
    let data = (try? JSONSerialization.data(withJSONObject: value)) ?? Data("{}".utf8)
    return strdup(String(data: data, encoding: .utf8) ?? "{}")
}

private func qgcNativeWindows() -> UnsafeMutablePointer<CChar>? {
    encode(NativeDebug.windows())
}

private func qgcNativeClick(_ title: UnsafePointer<CChar>?, _ x: Double, _ y: Double) -> UnsafeMutablePointer<CChar>? {
    encode(NativeDebug.click(window: title.map { String(cString: $0) } ?? "", x: x, y: y))
}

private func qgcNativeType(_ title: UnsafePointer<CChar>?, _ text: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>? {
    encode(NativeDebug.type(window: title.map { String(cString: $0) } ?? "",
                            text: text.map { String(cString: $0) } ?? ""))
}

private func qgcNativeProbe(_ id: UnsafePointer<CChar>?, _ action: UnsafePointer<CChar>?,
                           _ argsJson: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>? {
    let raw = argsJson.map { String(cString: $0) } ?? "{}"
    let decoded = (try? JSONSerialization.jsonObject(with: Data(raw.utf8))) as? [String: Any] ?? [:]
    return encode(NativeDebug.probe(
        id: id.map { String(cString: $0) } ?? "",
        action: action.map { String(cString: $0) } ?? "",
        args: decoded.mapValues { "\($0)" }))
}

private func qgcNativeMenu() -> UnsafeMutablePointer<CChar>? {
    encode(NativeDebug.menu())
}

private func qgcNativeMenuInvoke(_ path: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>? {
    encode(NativeDebug.invokeMenu(path: path.map { String(cString: $0) } ?? ""))
}

private func qgcNativeBridgeStats() -> UnsafeMutablePointer<CChar>? {
    encode(NativeDebug.bridgeStats())
}

extension NativeDebug {
    static func install() {
        var hooks = QGCNativeDebugHooks(
            windows: qgcNativeWindows,
            click: qgcNativeClick,
            type: qgcNativeType,
            probe: qgcNativeProbe,
            menu: qgcNativeMenu,
            menu_invoke: qgcNativeMenuInvoke,
            bridge_stats: qgcNativeBridgeStats)
        qgc_native_debug_install(&hooks)
    }
}
