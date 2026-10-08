import Combine
import Foundation
import QGCCore
import SwiftUI

final class PathSubject: ObservableObject {
    let path: String
    @Published fileprivate(set) var json: JSON?

    init(path: String, json: JSON?) {
        self.path = path
        self.json = json
    }
}

@MainActor
enum QgcWatch {
    nonisolated private static let client = "app"
    private static var counts: [String: Int] = [:]
    private static var subjects: [String: PathSubject] = [:]
    private static var installed = false
    static var sendWatch: (String) -> Void = { csv in offMainInOrder { qgc_bridge_watch_client(client, csv) } }

    static func subject(_ path: String) -> PathSubject {
        if let known = subjects[path] { return known }
        let made = PathSubject(path: path, json: nil)
        subjects[path] = made
        return made
    }

    static func retain(_ path: String) -> PathSubject {
        let held = counts[path, default: 0]
        counts[path] = held + 1
        if held == 0 { resend() }
        return subject(path)
    }

    static func release(_ path: String) {
        guard let held = counts[path] else { return }
        guard held <= 1 else { return counts[path] = held - 1 }
        counts[path] = nil
        subjects[path]?.json = nil
        resend()
    }

    static var watched: Set<String> { Set(counts.keys) }

    static func seed(_ path: String, _ json: JSON) {
        guard counts[path] != nil, let subject = subjects[path], subject.json == nil else { return }
        subject.json = json
    }

    private static func resend() {
        if !installed {
            installed = true
            qgc_bridge_set_event_handler(qgcWatchEvent)
        }
        sendWatch(counts.keys.sorted().joined(separator: ","))
    }

    fileprivate static func deliver(_ path: String, _ json: JSON) {
        guard counts[path] != nil, let subject = subjects[path], subject.json != json else { return }
        subject.json = json
    }
}

private func qgcWatchEvent(_ path: UnsafePointer<CChar>?, _ json: UnsafePointer<CChar>?) {
    guard let path, let json else { return }
    let key = String(cString: path)
    let value = JSON.parse(json)
    DispatchQueue.main.async { MainActor.assumeIsolated { QgcWatch.deliver(key, value) } }
}

@MainActor
final class PathWatcher: ObservableObject {
    private(set) var path: String?
    private var subject: PathSubject?
    private var forwarding: AnyCancellable?
    private var last: JSON?

    init(_ path: String? = nil) {
        follow(path)
    }

    deinit {
        let path = self.path
        if let path { MainActor.assumeIsolated { QgcWatch.release(path) } }
    }

    var value: JSON? { subject?.json }

    var holdingLast: JSON? {
        if let now = subject?.json { last = now }
        return last
    }

    func follow(_ next: String?) {
        guard next != path else { return }
        if let path { QgcWatch.release(path) }
        path = next
        subject = next.map(QgcWatch.retain)
        forwarding = subject?.objectWillChange.sink { [weak self] _ in self?.objectWillChange.send() }
    }
}

@propertyWrapper
struct QgcPath: DynamicProperty {
    @StateObject private var watcher = PathWatcher()
    private let path: String?
    private let holdLast: Bool

    init(_ path: String?, holdingLast: Bool = false) {
        self.path = path
        self.holdLast = holdingLast
    }

    var wrappedValue: JSON? { holdLast ? watcher.holdingLast : watcher.value }

    nonisolated func update() {
        MainActor.assumeIsolated { watcher.follow(path) }
    }
}

@propertyWrapper
struct QgcValue: DynamicProperty {
    @QgcPath private var json: JSON?

    init(_ path: String?) { _json = QgcPath(path) }

    var wrappedValue: JSON { json?["value"] ?? .null }
}

@propertyWrapper
struct QgcString: DynamicProperty {
    @QgcPath private var json: JSON?
    private let fallback: String

    init(_ path: String?, fallback: String = "") {
        _json = QgcPath(path)
        self.fallback = fallback
    }

    var wrappedValue: String { json?["value"].stringOrNil ?? fallback }
}

@propertyWrapper
struct QgcBool: DynamicProperty {
    @QgcPath private var json: JSON?

    init(_ path: String?) { _json = QgcPath(path) }

    var wrappedValue: Bool { json?["value"].truthy ?? false }
}

@propertyWrapper
struct QgcDouble: DynamicProperty {
    @QgcPath private var json: JSON?
    private let fallback: Double

    init(_ path: String?, fallback: Double = .nan) {
        _json = QgcPath(path)
        self.fallback = fallback
    }

    var wrappedValue: Double { json?["value"].double ?? fallback }
}

@propertyWrapper
struct QgcFacts: DynamicProperty {
    @QgcPath private var json: JSON?
    private let groupPath: String

    init(_ groupPath: String) {
        _json = QgcPath(groupPath)
        self.groupPath = groupPath
    }

    var wrappedValue: [Fact] { Qgc.facts(groupPath, json) }
}

@propertyWrapper
struct QgcStrings: DynamicProperty {
    @QgcPath private var json: JSON?

    init(_ path: String?) { _json = QgcPath(path) }

    var wrappedValue: [String] { json?["value"].strings ?? [] }
}

func truthy(_ value: JSON?) -> Bool { value?.truthy ?? false }
