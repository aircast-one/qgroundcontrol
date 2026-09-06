import AppKit
import Foundation

enum NativeDebug {
    // Driving the wrong window silently is worse than refusing: the caller cannot tell
    // a click that missed from one that landed somewhere unexpected.
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
                // AppKit frames are bottom-left origin; screencapture -R wants top-left
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

    // x/y are content-view coordinates with a top-left origin, which is how UI work is
    // described; CGEvent wants global display coordinates, also top-left.
    static func click(window title: String, x: Double, y: Double) -> [String: Any] {
        let window: NSWindow
        switch NativeDebug.resolve(title) {
        case let .found(match): window = match
        case let .failed(reason): return ["ok": false, "error": reason]
        }
        guard let content = window.contentView else {
            return ["ok": false, "error": "window has no content view"]
        }

        let screenHeight = NSScreen.screens.first?.frame.height ?? 0
        let contentInWindow = content.convert(content.bounds, to: nil)
        let originX = window.frame.origin.x + contentInWindow.origin.x
        let contentTopFromBottom = contentInWindow.origin.y + contentInWindow.height
        let originY = screenHeight - (window.frame.origin.y + contentTopFromBottom)
        let point = CGPoint(x: originX + x, y: originY + y)

        // Raising is asynchronous; posting into an unraised window lands on whatever
        // is actually in front. Let AppKit settle before synthesising the event.
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
        RunLoop.current.run(until: Date().addingTimeInterval(0.2))

        guard window.isKeyWindow else {
            return ["ok": false, "error": "window \(title) could not be raised"]
        }

        for type in [CGEventType.leftMouseDown, .leftMouseUp] {
            guard let event = CGEvent(mouseEventSource: nil, mouseType: type,
                                      mouseCursorPosition: point, mouseButton: .left) else {
                return ["ok": false, "error": "could not synthesise \(type.rawValue)"]
            }
            event.post(tap: .cghidEventTap)
        }

        return ["ok": true, "screenPoint": ["x": Int(point.x.rounded()), "y": Int(point.y.rounded())]]
    }

    // Types into whatever has focus in the named window. CGEvent carries the text as a
    // unicode string rather than keycodes, so layout and modifiers do not matter.
    static func type(window title: String, text: String) -> [String: Any] {
        let window: NSWindow
        switch NativeDebug.resolve(title) {
        case let .found(match): window = match
        case let .failed(reason): return ["ok": false, "error": reason]
        }
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
        RunLoop.current.run(until: Date().addingTimeInterval(0.2))
        guard window.isKeyWindow else {
            return ["ok": false, "error": "window \(title) could not be raised"]
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

    // path is slash-separated titles, e.g. "Window/Native Telemetry"
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

@_cdecl("qgc_native_windows")
public func qgcNativeWindows() -> UnsafeMutablePointer<CChar>? {
    encode(NativeDebug.windows())
}

@_cdecl("qgc_native_click")
public func qgcNativeClick(_ title: UnsafePointer<CChar>?, _ x: Double, _ y: Double) -> UnsafeMutablePointer<CChar>? {
    encode(NativeDebug.click(window: title.map { String(cString: $0) } ?? "", x: x, y: y))
}

@_cdecl("qgc_native_type")
public func qgcNativeType(_ title: UnsafePointer<CChar>?, _ text: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>? {
    encode(NativeDebug.type(window: title.map { String(cString: $0) } ?? "",
                            text: text.map { String(cString: $0) } ?? ""))
}

@_cdecl("qgc_native_menu")
public func qgcNativeMenu() -> UnsafeMutablePointer<CChar>? {
    encode(NativeDebug.menu())
}

@_cdecl("qgc_native_menu_invoke")
public func qgcNativeMenuInvoke(_ path: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>? {
    encode(NativeDebug.invokeMenu(path: path.map { String(cString: $0) } ?? ""))
}

@_cdecl("qgc_native_bridge_stats")
public func qgcNativeBridgeStats() -> UnsafeMutablePointer<CChar>? {
    encode(NativeDebug.bridgeStats())
}
