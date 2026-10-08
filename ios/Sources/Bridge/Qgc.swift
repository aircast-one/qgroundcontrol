import Foundation
import QGCCore
import os

struct FactSlider: Equatable {
    let from: Double
    let to: Double
    let decimals: Int
    let hint: String
}

struct Fact: Equatable, Identifiable {
    var path: String
    var name: String
    var description: String
    var units: String
    var valueString: String
    var value: JSON
    var enumStrings: [String]
    var enumValues: [String] = []
    var enumIndex: Int
    var unknownEnumLabel: String = ""
    var controlKind: String = ""
    var bitmaskStrings: [String] = []
    var bitmaskValues: [Int64] = []
    var isBool: Bool
    var isString: Bool
    var wholeNumbersOnly: Bool = false
    var readOnly: Bool
    var enabled: Bool = true
    var disabledReason: String = ""
    var minString: String = ""
    var maxString: String = ""
    var minIsDefaultForType: Bool = true
    var maxIsDefaultForType: Bool = true
    var defaultValueString: String = ""
    var vehicleRebootRequired: Bool = false
    var qgcRebootRequired: Bool = false
    var warning: Bool = false
    var changedFromDefault: Bool = false
    var optional: Bool = false
    var longDescription: String = ""
    var category: String = ""
    var group: String = ""
    var slider: FactSlider? = nil
    var firstEntryIsAll: Bool = false
    var indent: Bool = false
    var smallFont: Bool = false
    var shortLabel: String = ""
    var keywords: String = ""
    var inverted: Bool = false
    var rawChoice: Bool = false
    var valueDetails: String = ""
    var problem: String = ""
    var enumGroups: [String] = []

    var id: String { path }

    var title: String { (description.isBlank ? name : description).trimmingTrailing(":", " ") }
    var heading: String { shortLabel.trimmingTrailing(":", " ").ifBlank(title) }
    var detail: String {
        guard !shortLabel.isBlank, !enumStrings.contains(description), !restates(description, heading) else { return "" }
        return labelCase(description)
    }
    var isEnum: Bool { !enumStrings.isEmpty && bitmaskStrings.isEmpty }
    var isBitmask: Bool { !bitmaskStrings.isEmpty && bitmaskStrings.count == bitmaskValues.count }
    var valueIsOffTheEnumList: Bool {
        !unknownEnumLabel.isBlank && enumStrings.indices.contains(enumIndex) && enumStrings[enumIndex] == unknownEnumLabel
    }
    var boolValue: Bool {
        (value == .bool(true) || valueString.lowercased() == "true" || valueString == "1") != inverted
    }
    var acceptsWrite: Bool { !readOnly && enabled }
    var optionalSet: Bool { value.double.map { !$0.isNaN } ?? false }
}

func labelCase(_ note: String) -> String {
    !note.trimmingCharacters(in: .whitespaces).hasSuffix(".") && note.split(separator: " ").count <= 4 ? sentenceCase(note) : note
}

private func restates(_ note: String, _ label: String) -> Bool {
    let it = note.trimmingTrailing(".").lowercased()
    let label = label.lowercased()
    return !it.isEmpty && !label.isEmpty
        && (label.hasPrefix(it) || it.hasPrefix(label) || it.hasSuffix(" " + label) || label.hasSuffix(" " + it))
}

enum Qgc {
    private static let log = Logger(subsystem: "one.aircast.app", category: "QgcBridge")
    private static let slowCallMs = 250.0

    private static func timed<T>(_ what: String, _ block: () -> T) -> T {
        if Thread.isMainThread {
            log.warning("bridge call on the main thread: \(what, privacy: .public) - wrap it in offMain")
        }
        let started = DispatchTime.now().uptimeNanoseconds
        let result = block()
        let took = Double(DispatchTime.now().uptimeNanoseconds - started) / 1_000_000
        if took >= slowCallMs {
            log.warning("bridge call blocked \(took, privacy: .public)ms in \(what, privacy: .public)")
        }
        return result
    }

    private static func taken(_ raw: UnsafeMutablePointer<CChar>?) -> JSON {
        guard let raw else { return .null }
        defer { qgc_bridge_free(raw) }
        return JSON.parse(raw)
    }

    static func get(_ path: String) -> JSON {
        timed("get \(path)") { taken(qgc_bridge_get(path)) }
    }

    static func get(_ path: String, fields: [String]) -> JSON {
        timed("get \(path)") { taken(qgc_bridge_get_fields(path, fields.joined(separator: ","))) }
    }

    private static func write(_ path: String, _ payload: [String: Any?]) -> JSON {
        let body = JSON.encode(payload.mapValues { $0 ?? NSNull() })
        return timed("set \(path)") { taken(qgc_bridge_set(path, body)) }
    }

    @discardableResult
    static func set(_ path: String, _ value: Any?) -> Bool {
        let reply = write(path, ["value": value])
        if reply["ok"].bool { return true }
        log.warning("set \(path, privacy: .public) failed: \(reply["reason"].string.ifBlank("the bridge rejected the write"), privacy: .public)")
        return false
    }

    static func writeRefusal(_ path: String, _ value: Any?) -> String? {
        refusal(write(path, ["value": value]))
    }

    static func writeForVehicleRefusal(_ path: String, _ value: Any?, vehicle: Int) -> String? {
        refusal(write(path, ["value": value, "vehicle": vehicle]))
    }

    static func writeForcedRefusal(_ path: String, _ value: Any?) -> String? {
        refusal(write(path, ["value": value, "force": true]))
    }

    @discardableResult
    static func invoke(_ path: String, _ args: Any?...) -> Bool {
        let ok = call(path, arguments: args)?["ok"].bool ?? false
        if !ok { log.warning("invoke \(path, privacy: .public) failed") }
        return ok
    }

    static func invokeResult(_ path: String, _ args: Any?...) -> JSON {
        call(path, arguments: args)?["result"] ?? .null
    }

    static func refusalOf(_ path: String, _ args: Any?...) -> String? {
        refusal(call(path, arguments: args))
    }

    static func call(_ path: String, _ args: Any?...) -> JSON? {
        call(path, arguments: args)
    }

    static func call(_ path: String, arguments args: [Any?]) -> JSON? {
        let body = JSON.encode(args.map { $0 ?? NSNull() })
        let reply = timed("invoke \(path)") { taken(qgc_bridge_invoke(path, body)) }
        return reply.isNull ? nil : reply
    }

    static func facts(_ groupPath: String, _ json: JSON?) -> [Fact] {
        (json?["facts"].arrayOrNil ?? []).filter { $0.object != nil }.map { fact(groupPath, $0) }
    }

    static func factAt(_ path: String, _ json: JSON) -> Fact {
        var made = fact("", json)
        made.path = path
        return made
    }

    static func fact(_ groupPath: String, _ json: JSON) -> Fact {
        let name = json["name"].string
        return Fact(
            path: groupPath.isEmpty ? name : "\(groupPath).\(name)",
            name: name,
            description: json["shortDescription"].string,
            units: json["units"].string,
            valueString: json["valueString"].string,
            value: json["value"],
            enumStrings: json["enumStrings"].strings,
            enumValues: json["enumValues"].strings,
            enumIndex: json["enumIndex"].int(-1),
            unknownEnumLabel: json["unknownEnumLabel"].string,
            bitmaskStrings: json["bitmaskStrings"].strings,
            bitmaskValues: json["bitmaskValues"].array.map { $0.int64 ?? 0 },
            isBool: json["typeIsBool"].bool,
            isString: json["typeIsString"].bool,
            wholeNumbersOnly: json["typeIsInteger"].bool,
            readOnly: json["readOnly"].bool,
            minString: json["minString"].string,
            maxString: json["maxString"].string,
            minIsDefaultForType: json["minIsDefaultForType"].bool(true),
            maxIsDefaultForType: json["maxIsDefaultForType"].bool(true),
            defaultValueString: json["defaultValueString"].string,
            vehicleRebootRequired: json["vehicleRebootRequired"].bool,
            qgcRebootRequired: json["qgcRebootRequired"].bool,
            changedFromDefault: json["defaultValueAvailable"].bool && !json["valueEqualsDefault"].bool(true),
            longDescription: json["longDescription"].string,
            category: json["category"].string,
            group: json["group"].string
        )
    }
}

func refusal(_ answer: JSON?) -> String? {
    guard let answer else { return "The vehicle did not answer." }
    if answer["ok"].bool { return nil }
    return answer["reason"].string.ifBlank("The vehicle refused.")
}

func settingControl(_ factPath: String) -> String { "view.control(\(factPath))" }

func offMain(_ block: @escaping @Sendable () -> Void) {
    DispatchQueue.global(qos: .userInitiated).async(execute: block)
}

private let inOrder = DispatchQueue(label: "one.aircast.bridge.in-order", qos: .userInitiated)

func offMainInOrder(_ block: @escaping @Sendable () -> Void) {
    inOrder.async(execute: block)
}

func offMain<T: Sendable>(_ block: @escaping @Sendable () -> T) async -> T {
    await withCheckedContinuation { done in
        DispatchQueue.global(qos: .userInitiated).async { done.resume(returning: block()) }
    }
}

func onMain(_ block: @escaping @MainActor () -> Void) {
    DispatchQueue.main.async { MainActor.assumeIsolated(block) }
}
