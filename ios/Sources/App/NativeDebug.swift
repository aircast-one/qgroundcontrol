#if os(macOS)
import AppKit
import ApplicationServices
import QGCCore

private let UI_PROBE = "ui"
private let DEBUG_UI_PROBE = "debugUi"
private let SNAPSHOT_PROBE = "snapshot"
private let PROBES = [UI_PROBE, DEBUG_UI_PROBE, SNAPSHOT_PROBE]
private let MAX_UI_DEPTH = 64
private let AX_TIMEOUT_SECONDS: Float = 5

@MainActor
enum NativeDebug {
    static func install() {
        var hooks = QGCNativeDebugHooks(
            windows: { encode(onMain(NativeDebug.windows)) },
            click: { title, x, y in
                let window = string(title)
                return encode(onMain { NativeDebug.click(window: window, x: x, y: y) })
            },
            type: { title, text in
                let (window, typed) = (string(title), string(text))
                return encode(onMain { NativeDebug.type(window: window, text: typed) })
            },
            probe: { id, action, argsJson in
                let decoded = (try? JSONSerialization.jsonObject(with: Data(string(argsJson).utf8))) as? [String: Any] ?? [:]
                let (probeId, probeAction, args) = (string(id), string(action), decoded.mapValues { "\($0)" })
                return encode(NativeDebug.probe(id: probeId, action: probeAction, args: args))
            },
            menu: { encode(onMain(NativeDebug.menu)) },
            menu_invoke: { path in
                let item = string(path)
                return encode(onMain { NativeDebug.invokeMenu(path: item) })
            },
            bridge_stats: nil)
        qgc_native_debug_install(&hooks)
    }

    nonisolated static var screenIsLocked: Bool {
        let session = CGSessionCopyCurrentDictionary() as? [String: Any]
        return (session?["CGSSessionScreenIsLocked"] as? NSNumber)?.boolValue ?? false
    }

    static func windows() -> [String: Any] {
        let screenHeight = NSScreen.screens.first?.frame.height ?? 0
        let entries = NSApp.windows.enumerated().map { index, window -> [String: Any] in
            let frame = window.frame
            return [
                "index": index,
                "title": window.title,
                "number": window.windowNumber,
                "visible": window.isVisible,
                "onActiveSpace": window.isOnActiveSpace,
                "key": window.isKeyWindow,
                "main": window.isMainWindow,
                "sheet": window.isSheet,
                "captureRect": [
                    "x": Int(frame.origin.x.rounded()),
                    "y": Int((screenHeight - frame.origin.y - frame.height).rounded()),
                    "width": Int(frame.width.rounded()),
                    "height": Int(frame.height.rounded()),
                ],
            ]
        }
        return ["windows": entries, "screenHeight": Int(screenHeight.rounded())]
    }

    private static func resolve(_ title: String) -> Result<NSWindow, ProbeError> {
        if let index = windowIndex(title) {
            return NSApp.windows.indices.contains(index) ? .success(NSApp.windows[index]) : .failure(ProbeError("no window #\(index)"))
        }
        let visible = NSApp.windows.filter(\.isVisible)
        let matches = title.isEmpty ? visible.filter(\.isKeyWindow) + visible.filter(\.isMainWindow) : visible.filter { $0.title == title }
        switch (title.isEmpty, matches.count) {
        case (true, 0): return visible.first.map { .success($0) } ?? .failure(ProbeError("no visible window"))
        case (true, _), (false, 1): return .success(matches[0])
        case (false, 0): return .failure(ProbeError("no visible window titled \(title)"))
        default: return .failure(ProbeError("\(matches.count) visible windows are titled \(title)"))
        }
    }

    private static func raise(_ window: NSWindow) -> Bool {
        (0..<10).contains { _ in
            NSApp.activate(ignoringOtherApps: true)
            window.makeKeyAndOrderFront(nil)
            RunLoop.current.run(until: Date().addingTimeInterval(0.15))
            return window.isKeyWindow
        }
    }

    static func click(window title: String, x: Double, y: Double) -> [String: Any] {
        guard !screenIsLocked else {
            return refusal("screen is locked; synthesised clicks cannot reach a window that cannot become key — press elements through /native/probe?id=ui instead")
        }
        switch resolve(title) {
        case .failure(let error): return refusal(error.message)
        case .success(let window): return click(window, x: x, y: y)
        }
    }

    static func click(_ window: NSWindow, x: Double, y: Double) -> [String: Any] {
        let size = window.frame.size
        guard (0...size.width).contains(x), (0...size.height).contains(y) else {
            return refusal("point (\(x), \(y)) is outside the \(Int(size.width))x\(Int(size.height)) window")
        }
        let point = NSPoint(x: x, y: size.height - y)
        let sent = [NSEvent.EventType.leftMouseDown, .leftMouseUp].allSatisfy { type in
            NSEvent.mouseEvent(
                with: type, location: point, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
                windowNumber: window.windowNumber, context: nil, eventNumber: 0, clickCount: 1,
                pressure: type == .leftMouseDown ? 1 : 0
            ).map { window.sendEvent($0) } != nil
        }
        return sent ? ["ok": true, "windowPoint": ["x": Int(point.x), "y": Int(point.y)]] : refusal("could not synthesise a mouse event")
    }

    static func type(window title: String, text: String) -> [String: Any] {
        guard !screenIsLocked else { return refusal("screen is locked; set a field through /native/probe?id=ui&action=set instead") }
        switch resolve(title) {
        case .failure(let error): return refusal(error.message)
        case .success(let window):
            guard raise(window) else { return refusal("window \(window.title) could not be raised; another app may hold focus") }
            let sent = text.map(String.init).allSatisfy { character in
                [NSEvent.EventType.keyDown, .keyUp].allSatisfy { type in
                    NSEvent.keyEvent(
                        with: type, location: .zero, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
                        windowNumber: window.windowNumber, context: nil, characters: character,
                        charactersIgnoringModifiers: character, isARepeat: false, keyCode: 0
                    ).map { window.sendEvent($0) } != nil
                }
            }
            RunLoop.current.run(until: Date().addingTimeInterval(0.2))
            return sent ? ["ok": true, "typed": text] : refusal("could not synthesise a key event")
        }
    }

    static func menu() -> [String: Any] {
        func describe(_ menu: NSMenu) -> [[String: Any]] {
            menu.update()
            return menu.items.map { item in
                [
                    "title": item.submenu.map(\.title).flatMap { $0.isEmpty ? nil : $0 } ?? item.title,
                    "enabled": item.isEnabled,
                    "separator": item.isSeparatorItem,
                    "keyEquivalent": item.keyEquivalent,
                    "checked": item.state == .on,
                ].merging(item.submenu.map { ["items": describe($0)] } ?? [:]) { first, _ in first }
            }
        }
        return ["menu": NSApp.mainMenu.map(describe) ?? []]
    }

    static func invokeMenu(path: String) -> [String: Any] {
        let parts = path.split(separator: "/").map(String.init)
        guard !parts.isEmpty, let bar = NSApp.mainMenu else { return refusal("empty path or no menu bar") }
        let found = parts.reduce(Result<(NSMenu?, NSMenuItem?), ProbeError>.success((bar, nil))) { walked, part in
            walked.flatMap { menu, _ in
                menu?.update()
                guard let item = menu?.items.first(where: { $0.title == part || $0.submenu?.title == part }) else {
                    return .failure(ProbeError("no menu item \(part) in \(path)"))
                }
                return .success((item.submenu, item))
            }
        }
        switch found {
        case .failure(let error): return refusal(error.message)
        case .success((_, let item)):
            guard let item, let menu = item.menu else { return refusal("unresolved \(path)") }
            guard item.isEnabled else { return refusal("\(path) is disabled") }
            menu.performActionForItem(at: menu.index(of: item))
            return ["ok": true, "invoked": path]
        }
    }

    nonisolated static func probe(id: String, action: String, args: [String: String]) -> [String: Any] {
        switch id {
        case "": ["screenIsLocked": screenIsLocked, "accessibilityTrusted": AXIsProcessTrusted(), "probes": PROBES]
        case UI_PROBE: action.isEmpty ? UiTree.read(args) : UiTree.act(action, args)
        case DEBUG_UI_PROBE: onMain { debugUi(action, args["value"]) }
        case SNAPSHOT_PROBE: onMain { snapshot(args["window"] ?? "") }
        default: refusal("no probe \(id)", ["probes": PROBES])
        }
    }

    private static func debugUi(_ action: String, _ value: String?) -> [String: Any] {
        guard !action.isEmpty else {
            return ["ok": true, "state": (try? JSONSerialization.jsonObject(with: Data(DebugUi.state.utf8))) ?? [:], "commands": DEBUG_COMMANDS]
        }
        switch debugCommand(action, value) {
        case .failure(let refused): return refusal(refused.message)
        case .success(let command):
            NotificationCenter.default.post(name: DebugUi.commands, object: command)
            return ["ok": true]
        }
    }

    private static func snapshot(_ title: String) -> [String: Any] {
        switch resolve(title) {
        case .failure(let error): return refusal(error.message)
        case .success(let window):
            guard let view = window.contentView?.superview ?? window.contentView,
                  let bitmap = view.bitmapImageRepForCachingDisplay(in: view.bounds)
            else { return refusal("window \(window.title) has nothing to draw") }
            view.cacheDisplay(in: view.bounds, to: bitmap)
            let file = FileManager.default.temporaryDirectory.appendingPathComponent("aircast-snapshot-\(window.windowNumber).png")
            guard let png = bitmap.representation(using: .png, properties: [:]), (try? png.write(to: file)) != nil else {
                return refusal("could not write \(file.path)")
            }
            return ["ok": true, "path": file.path, "width": bitmap.pixelsWide, "height": bitmap.pixelsHigh]
        }
    }
}

@objc private protocol Accessible {
    func accessibilityAttributeValue(_ attribute: NSAccessibility.Attribute) -> Any?
    func accessibilityActionNames() -> [NSAccessibility.Action]
    func accessibilityPerformAction(_ action: NSAccessibility.Action)
    func accessibilitySetValue(_ value: Any?, forAttribute attribute: NSAccessibility.Attribute)
    func accessibilityChildren() -> [Any]?
    func accessibilityRole() -> Any?
    func accessibilitySubrole() -> Any?
    func accessibilityLabel() -> Any?
    func accessibilityTitle() -> Any?
    func accessibilityValue() -> Any?
    func accessibilityPlaceholderValue() -> Any?
    func accessibilityIdentifier() -> Any?
    func accessibilityHelp() -> Any?
    func isAccessibilityEnabled() -> Bool
    func isAccessibilitySelected() -> Bool
    func isAccessibilityFocused() -> Bool
    func accessibilityFrame() -> NSRect
    func accessibilityPerformPress() -> Bool
    func accessibilityPerformIncrement() -> Bool
    func accessibilityPerformDecrement() -> Bool
    func accessibilityPerformShowMenu() -> Bool
    func accessibilityPerformCancel() -> Bool
    func accessibilityPerformConfirm() -> Bool
    func accessibilityPerformPick() -> Bool
    func setAccessibilityValue(_ value: Any?)
    func setAccessibilityFocused(_ focused: Bool)
    func setAccessibilitySelected(_ selected: Bool)
    func accessibilityParent() -> Any?
    func setAccessibilitySelectedRows(_ rows: [Any])
}

private let PERFORMS: [(action: NSAccessibility.Action, selector: Selector, run: (Accessible) -> Bool)] = [
    (.press, #selector(Accessible.accessibilityPerformPress), { $0.accessibilityPerformPress() }),
    (.increment, #selector(Accessible.accessibilityPerformIncrement), { $0.accessibilityPerformIncrement() }),
    (.decrement, #selector(Accessible.accessibilityPerformDecrement), { $0.accessibilityPerformDecrement() }),
    (.showMenu, #selector(Accessible.accessibilityPerformShowMenu), { $0.accessibilityPerformShowMenu() }),
    (.cancel, #selector(Accessible.accessibilityPerformCancel), { $0.accessibilityPerformCancel() }),
    (.confirm, #selector(Accessible.accessibilityPerformConfirm), { $0.accessibilityPerformConfirm() }),
    (.pick, #selector(Accessible.accessibilityPerformPick), { $0.accessibilityPerformPick() }),
]

private struct AxElement {
    let object: AnyObject

    private var ax: Accessible { unsafeBitCast(object, to: Accessible.self) }
    private func has(_ selector: Selector) -> Bool { (object as? NSObject)?.responds(to: selector) == true }

    private func attribute(_ name: NSAccessibility.Attribute) -> Any? {
        has(#selector(Accessible.accessibilityAttributeValue(_:))) ? ax.accessibilityAttributeValue(name) : nil
    }

    private func read<T>(_ selector: Selector, _ method: (Accessible) -> T?, _ name: NSAccessibility.Attribute) -> T? {
        has(selector) ? method(ax) : attribute(name) as? T
    }

    private func text(_ selector: Selector, _ method: (Accessible) -> Any?, _ name: NSAccessibility.Attribute) -> String? {
        let found = has(selector) ? method(ax) : attribute(name)
        return (found as? String) ?? (found as? NSAttributedString)?.string
    }

    var children: [AnyObject] {
        (has(#selector(Accessible.accessibilityChildren)) ? ax.accessibilityChildren() : attribute(.children) as? [Any]).map { $0 as [AnyObject] } ?? []
    }
    var role: String? { text(#selector(Accessible.accessibilityRole), { $0.accessibilityRole() }, .role) }
    var subrole: String? { text(#selector(Accessible.accessibilitySubrole), { $0.accessibilitySubrole() }, .subrole) }
    var label: String? { text(#selector(Accessible.accessibilityLabel), { $0.accessibilityLabel() }, .description) }
    var title: String? { text(#selector(Accessible.accessibilityTitle), { $0.accessibilityTitle() }, .title) }
    var value: Any? { has(#selector(Accessible.accessibilityValue)) ? ax.accessibilityValue() : attribute(.value) }
    var placeholder: String? { text(#selector(Accessible.accessibilityPlaceholderValue), { $0.accessibilityPlaceholderValue() }, .placeholderValue) }
    var identifier: String? { text(#selector(Accessible.accessibilityIdentifier), { $0.accessibilityIdentifier() }, .identifier) }
    var help: String? { text(#selector(Accessible.accessibilityHelp), { $0.accessibilityHelp() }, .help) }
    var enabled: Bool { read(#selector(Accessible.isAccessibilityEnabled), { $0.isAccessibilityEnabled() }, .enabled) ?? true }
    var selected: Bool { read(#selector(Accessible.isAccessibilitySelected), { $0.isAccessibilitySelected() }, .selected) ?? false }
    var focused: Bool { read(#selector(Accessible.isAccessibilityFocused), { $0.isAccessibilityFocused() }, .focused) ?? false }

    var frame: NSRect {
        if has(#selector(Accessible.accessibilityFrame)) { return ax.accessibilityFrame() }
        let origin = (attribute(.position) as? NSValue)?.pointValue ?? .zero
        return NSRect(origin: origin, size: (attribute(.size) as? NSValue)?.sizeValue ?? .zero)
    }

    var actionable: Bool { role.map(Self.actionableRoles.contains) == true }

    var actions: [NSAccessibility.Action] {
        let named = has(#selector(Accessible.accessibilityActionNames)) ? ax.accessibilityActionNames() : []
        let stepped = [NSAccessibility.Role.slider.rawValue, NSAccessibility.Role.incrementor.rawValue].contains(role ?? "")
        return !named.isEmpty || !actionable ? named : stepped ? [.increment, .decrement] : [.press]
    }

    private static let actionableRoles: Set<String> = [
        NSAccessibility.Role.button, .checkBox, .radioButton, .popUpButton, .menuButton, .menuItem, .slider,
        .incrementor, .disclosureTriangle, .link, .cell, .row, .textField, .comboBox, .colorWell,
    ].map(\.rawValue).reduce(into: []) { $0.insert($1) }

    func perform(_ action: NSAccessibility.Action) -> Bool {
        if let modern = PERFORMS.first(where: { $0.action == action && has($0.selector) }) {
            return modern.run(ax)
        }
        guard has(#selector(Accessible.accessibilityPerformAction(_:))) else { return false }
        ax.accessibilityPerformAction(action)
        return true
    }

    func set(_ newValue: Any) -> Bool {
        if has(#selector(Accessible.setAccessibilityValue(_:))) { ax.setAccessibilityValue(newValue); return true }
        guard has(#selector(Accessible.accessibilitySetValue(_:forAttribute:))) else { return false }
        ax.accessibilitySetValue(newValue, forAttribute: .value)
        return true
    }

    func select() -> Bool {
        let table = role == NSAccessibility.Role.row.rawValue
            ? (has(#selector(Accessible.accessibilityParent)) ? ax.accessibilityParent() : attribute(.parent)) as AnyObject?
            : nil
        if let table, (table as? NSObject)?.responds(to: #selector(Accessible.setAccessibilitySelectedRows(_:))) == true {
            unsafeBitCast(table, to: Accessible.self).setAccessibilitySelectedRows([object])
            return true
        }
        if has(#selector(Accessible.setAccessibilitySelected(_:))) { ax.setAccessibilitySelected(true); return true }
        guard has(#selector(Accessible.accessibilitySetValue(_:forAttribute:))) else { return false }
        ax.accessibilitySetValue(true, forAttribute: .selected)
        return true
    }

    func focus() -> Bool {
        if has(#selector(Accessible.setAccessibilityFocused(_:))) { ax.setAccessibilityFocused(true); return true }
        guard has(#selector(Accessible.accessibilitySetValue(_:forAttribute:))) else { return false }
        ax.accessibilitySetValue(true, forAttribute: .focused)
        return true
    }
}

enum UiTree {
    struct Node {
        let path: String
        fileprivate let element: AxElement
        let window: NSWindow
    }

    nonisolated static func read(_ args: [String: String]) -> [String: Any] {
        wake()
        return onMain {
            let needle = args["filter"]?.lowercased() ?? ""
            let entries = nodes(args["window"]).map(describe).filter { entry in
                let actions = entry["actions"] as? [String] ?? []
                let text = ["label", "title", "value", "id", "help"].compactMap { entry[$0] as? String }
                let wanted = needle.isEmpty || (text + [entry["role"] as? String ?? ""]).contains { $0.lowercased().contains(needle) }
                let shown = args["all"] == "1" || !text.isEmpty || !actions.isEmpty
                return wanted && shown && (args["actionable"] != "1" || !actions.isEmpty)
            }
            return ["ok": true, "count": entries.count, "nodes": entries]
        }
    }

    nonisolated static func act(_ action: String, _ args: [String: String]) -> [String: Any] {
        wake()
        return onMain {
            switch target(args) {
            case .failure(let error): return refusal(error.message)
            case .success(let node):
                let element = node.element
                let done: Bool
                switch action {
                case "set":
                    guard let text = args["value"] else { return refusal("set needs value=") }
                    done = element.set(element.value is NSNumber ? Double(text).map { $0 as NSNumber } ?? text : text)
                case "focus":
                    done = element.focus()
                case "select":
                    done = element.select()
                case "escape":
                    let key = NSEvent.keyEvent(
                        with: .keyDown, location: .zero, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
                        windowNumber: node.window.windowNumber, context: nil, characters: "\u{1b}",
                        charactersIgnoringModifiers: "\u{1b}", isARepeat: false, keyCode: 53)
                    done = key.map(node.window.performKeyEquivalent) == true
                        || (node.window.firstResponder ?? node.window).tryToPerform(#selector(NSResponder.cancelOperation(_:)), with: nil)
                case "click":
                    let box = describe(node)["frame"] as? [Int] ?? []
                    guard box.count == 4 else { return refusal("\(node.path) has no frame") }
                    let clicked = NativeDebug.click(node.window, x: Double(box[0]) + Double(box[2]) / 2, y: Double(box[1]) + Double(box[3]) / 2)
                    done = clicked["ok"] as? Bool == true
                default:
                    let name = NSAccessibility.Action(rawValue: action.hasPrefix("AX") ? action : "AX" + action.prefix(1).uppercased() + action.dropFirst())
                    guard element.actions.contains(name) else {
                        return refusal("\(node.path) has no \(name.rawValue) action", ["element": describe(node)])
                    }
                    done = element.perform(name)
                }
                return done ? ["ok": true, "element": describe(node)] : refusal("\(node.path) refused \(action)", ["element": describe(node)])
            }
        }
    }

    private nonisolated static func wake() {
        var children: CFTypeRef?
        _ = AXUIElementCopyAttributeValue(AXUIElementCreateApplication(getpid()), kAXChildrenAttribute as CFString, &children)
    }

    @MainActor
    private static func nodes(_ windowTitle: String?) -> [Node] {
        func walk(_ element: AxElement, _ path: String, _ window: NSWindow, _ depth: Int) -> [Node] {
            let children = depth < MAX_UI_DEPTH ? element.children : []
            return [Node(path: path, element: element, window: window)] + children.enumerated().flatMap { index, child in
                walk(AxElement(object: child), "\(path)/\(index)", window, depth + 1)
            }
        }
        return NSApp.windows.enumerated()
            .filter { index, window in
                window.isVisible && (windowTitle.flatMap { $0.isEmpty ? nil : $0 }.map { $0 == window.title || windowIndex($0) == index } ?? !window.isSheet)
            }
            .flatMap { index, window in walk(AxElement(object: window), "\(index)", window, 0) }
    }

    @MainActor
    private static func target(_ args: [String: String]) -> Result<Node, ProbeError> {
        let all = nodes(args["window"])
        if let path = args["path"] {
            return all.first { $0.path == path }.map { .success($0) } ?? .failure(ProbeError("no element at \(path); the tree may have changed, read it again"))
        }
        guard let wanted = args["label"] else { return .failure(ProbeError("path or label is required")) }
        let matches = all.filter { node in
            let entry = describe(node)
            return ["label", "title", "id", "value"].contains { entry[$0] as? String == wanted } && (args["role"].map { entry["role"] as? String == $0 } ?? true)
        }
        let nth = Int(args["nth"] ?? "") ?? (matches.count == 1 ? 0 : -1)
        guard matches.indices.contains(nth) else {
            return .failure(ProbeError(matches.isEmpty
                ? "no element labelled \(wanted)\(args["role"].map { " with role \($0)" } ?? "")"
                : "\(matches.count) elements are labelled \(wanted) (\(matches.map(\.path).joined(separator: ", "))); pass nth= or path="))
        }
        return .success(matches[nth])
    }

    @MainActor
    static func describe(_ node: Node) -> [String: Any] {
        let element = node.element
        let box = element.frame
        let window = node.window.frame
        let current = element.value
        let edges = [box.minX - window.minX, window.maxY - box.maxY, box.width, box.height]
        let fields: [String: Any?] = [
            "path": node.path,
            "role": element.role,
            "subrole": element.subrole,
            "label": element.label,
            "title": element.title,
            "value": (current as? String) ?? (current as? NSNumber).map(\.stringValue) ?? (current as? NSAttributedString)?.string,
            "placeholder": element.placeholder,
            "id": element.identifier,
            "help": element.help,
            "enabled": element.enabled || !element.actionable ? nil : false,
            "selected": element.selected ? true : nil,
            "focused": element.focused ? true : nil,
            "frame": edges.allSatisfy(\.isFinite) ? edges.map { Int($0.rounded()) } : nil,
            "actions": element.actions.map { $0.rawValue.hasPrefix("AX") ? String($0.rawValue.dropFirst(2)) : $0.rawValue },
        ]
        return fields.compactMapValues { value in
            switch value {
            case let text as String: text.isEmpty ? nil : text
            case let list as [String]: list.isEmpty ? nil : list
            default: value
            }
        }
    }
}

private func windowIndex(_ name: String) -> Int? {
    name.hasPrefix("#") ? Int(name.dropFirst()) : nil
}

struct ProbeError: Error {
    let message: String
    init(_ message: String) { self.message = message }
}

private func refusal(_ message: String, _ extra: [String: Any] = [:]) -> [String: Any] {
    ["ok": false, "error": message].merging(extra) { first, _ in first }
}

private func string(_ text: UnsafePointer<CChar>?) -> String {
    text.map { String(cString: $0) } ?? ""
}

private func onMain(_ body: @escaping @MainActor () -> [String: Any]) -> [String: Any] {
    guard !Thread.isMainThread else { return MainActor.assumeIsolated(body) }
    let answer = Answer()
    let done = DispatchSemaphore(value: 0)
    DispatchQueue.main.async {
        answer.value = MainActor.assumeIsolated(body)
        done.signal()
    }
    return done.wait(timeout: .now() + .seconds(15)) == .success ? answer.value : ["ok": false, "error": "the main thread did not answer in 15s"]
}

private final class Answer: @unchecked Sendable {
    var value: [String: Any] = [:]
}

private func encode(_ value: [String: Any]) -> UnsafeMutablePointer<CChar>? {
    let data = (try? JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])) ?? Data("{}".utf8)
    return strdup(String(decoding: data, as: UTF8.self))
}
#endif
