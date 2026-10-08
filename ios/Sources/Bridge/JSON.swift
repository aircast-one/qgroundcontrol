import Foundation

enum JSON: Equatable, Hashable, Sendable {
    case null
    case bool(Bool)
    case number(Double)
    case string(String)
    case array([JSON])
    case object([String: JSON])

    init(_ raw: Any?) {
        switch raw {
        case nil, is NSNull: self = .null
        case let number as NSNumber where CFGetTypeID(number) == CFBooleanGetTypeID(): self = .bool(number.boolValue)
        case let number as NSNumber: self = .number(number.doubleValue)
        case let text as String: self = .string(text)
        case let list as [Any]: self = .array(list.map(JSON.init))
        case let map as [String: Any]: self = .object(map.mapValues(JSON.init))
        case let value as JSON: self = value
        case let value as Bool: self = .bool(value)
        case let value as Int: self = .number(Double(value))
        case let value as Double: self = .number(value)
        default: self = .null
        }
    }

    static func parse(_ text: String) -> JSON {
        guard let data = text.data(using: .utf8),
              let raw = try? JSONSerialization.jsonObject(with: data, options: [.fragmentsAllowed]) else { return .null }
        return JSON(raw)
    }

    static func parse(_ raw: UnsafePointer<CChar>) -> JSON {
        let data = Data(bytes: raw, count: strlen(raw))
        guard let parsed = try? JSONSerialization.jsonObject(with: data, options: [.fragmentsAllowed]) else { return .null }
        return JSON(parsed)
    }

    subscript(key: String) -> JSON {
        if case .object(let map) = self { return map[key] ?? .null }
        return .null
    }

    subscript(index: Int) -> JSON {
        if case .array(let list) = self, list.indices.contains(index) { return list[index] }
        return .null
    }

    var isNull: Bool { self == .null }

    func has(_ key: String) -> Bool {
        if case .object(let map) = self { return map[key].map { !$0.isNull } ?? false }
        return false
    }

    var object: [String: JSON]? {
        if case .object(let map) = self { return map }
        return nil
    }

    var array: [JSON] {
        if case .array(let list) = self { return list }
        return []
    }

    var arrayOrNil: [JSON]? {
        if case .array(let list) = self { return list }
        return nil
    }

    var keys: [String] { object.map { Array($0.keys) } ?? [] }

    var stringOrNil: String? {
        switch self {
        case .string(let text): text
        case .number(let value): JSON.format(value)
        case .bool(let value): value ? "true" : "false"
        default: nil
        }
    }

    var string: String { stringOrNil ?? "" }

    func string(_ fallback: String) -> String { stringOrNil ?? fallback }

    var double: Double? {
        switch self {
        case .number(let value): value
        case .bool(let value): value ? 1 : 0
        case .string(let text): Double(text)
        default: nil
        }
    }

    func double(_ fallback: Double) -> Double { double ?? fallback }

    var int: Int? { double.flatMap { $0.isFinite ? Int(exactly: $0.rounded(.towardZero)) : nil } }

    func int(_ fallback: Int) -> Int { int ?? fallback }

    var int64: Int64? { double.flatMap { $0.isFinite ? Int64(exactly: $0.rounded(.towardZero)) : nil } }

    var boolOrNil: Bool? {
        switch self {
        case .bool(let value): value
        case .number(let value): value != 0
        case .string(let text): text.lowercased() == "true" ? true : text.lowercased() == "false" ? false : nil
        default: nil
        }
    }

    var bool: Bool { boolOrNil ?? false }

    func bool(_ fallback: Bool) -> Bool { boolOrNil ?? fallback }

    var truthy: Bool {
        switch self {
        case .bool(let value): value
        case .number(let value): value != 0
        case .string(let text): text.lowercased() == "true" || (Double(text).map { $0 != 0 } ?? false)
        default: false
        }
    }

    var strings: [String] { array.map(\.string) }

    var any: Any {
        switch self {
        case .null: NSNull()
        case .bool(let value): value
        case .number(let value) where !value.isFinite: NSNull()
        case .number(let value): value.rounded() == value && abs(value) < 9.0e15 ? Int64(value) as Any : value
        case .string(let text): text
        case .array(let list): list.map(\.any)
        case .object(let map): map.mapValues(\.any)
        }
    }

    var text: String {
        return (try? JSONSerialization.data(withJSONObject: any, options: [.sortedKeys, .fragmentsAllowed]))
            .flatMap { String(data: $0, encoding: .utf8) } ?? "null"
    }

    static func format(_ value: Double) -> String {
        value.rounded() == value && abs(value) < 1e15 ? String(Int64(value)) : String(value)
    }

    static func encode(_ value: Any?) -> String {
        JSON(value).text
    }
}
