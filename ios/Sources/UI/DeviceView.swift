import Foundation
import SwiftUI

enum DeviceLevel {
    static let Ready = 0
    static let Warn = 1
    static let Blocked = 2
    static let Neutral = 3
}

let DEVICE_PAGE = "Aircast device"
let DEVICE_CONNECT = "device.connect"
let DEVICE_DISCONNECT = "device.disconnect"
private let DEVICE_CALL = "device.call"
private let DEVICE_READ = "device.read"
private let DEVICE_UPLOAD = "device.upload"
private let DEVICE_QR = "device.qr"
private let DEVICE_REACH = "device.reach"

func deviceViewPath(_ page: String) -> String { "view.device(\(page))" }

private let URI_UNRESERVED = CharacterSet(charactersIn: "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789.-*_")

func uriComponent(_ text: String) -> String { text.addingPercentEncoding(withAllowedCharacters: URI_UNRESERVED) ?? text }

struct DeviceReading: Equatable {
    let origin: String
    let host: String
    let answered: Bool
    let view: JSON?
}

func deviceReading(_ json: JSON?) -> DeviceReading {
    DeviceReading(
        origin: json?["origin"].string ?? "",
        host: json?["host"].string ?? "",
        answered: json?["answered"].bool ?? false,
        view: json?["view"].objectOrNil
    )
}

@propertyWrapper
struct DeviceView: DynamicProperty {
    @QgcPath private var json: JSON?

    init(_ page: String) { _json = QgcPath(deviceViewPath(page)) }

    var wrappedValue: DeviceReading { deviceReading(json) }
}

func deviceView<T>(_ reading: DeviceReading, _ parse: (JSON) -> T?) -> T? { reading.view.flatMap(parse) }

extension EnvironmentValues {
    @Entry var LocalDeviceOrigin = ""
    @Entry var LocalDeviceScroll: ScrollViewProxy? = nil
}

func resolveDeviceUrl(_ origin: String, _ path: String) -> String {
    guard !origin.isEmpty, let base = URL(string: origin), let resolved = URL(string: path, relativeTo: base) else { return path }
    return resolved.absoluteString
}

struct DeviceRefusal: Equatable, Sendable {
    let message: String
    let field: String
}

enum DeviceMethod: String, Sendable { case POST, PUT, DELETE }

let DEVICE_NO_ANSWER = DeviceRefusal(message: "The device did not answer.", field: "")

func deviceRefusal(_ answer: JSON?) -> DeviceRefusal? {
    guard let answer else { return DEVICE_NO_ANSWER }
    if answer["ok"].bool { return nil }
    return DeviceRefusal(message: answer["reason"].string.ifBlank(DEVICE_NO_ANSWER.message), field: answer["field"].string)
}

struct DeviceAnswer: Equatable, Sendable {
    let refusal: DeviceRefusal?
    let result: JSON?
}

func deviceAnswer(_ answer: JSON?) -> DeviceAnswer { DeviceAnswer(refusal: deviceRefusal(answer), result: answer?["result"].objectOrNil) }

func deviceBody(_ fields: [String: String]) -> JSON { .object(fields.mapValues { .string($0) }) }

func deviceSend(_ method: DeviceMethod, _ path: String, _ body: JSON? = nil) async -> DeviceAnswer {
    await offMain { deviceAnswer(Qgc.call(DEVICE_CALL, method.rawValue, path, body ?? JSON.null)) }
}

func deviceCall(_ method: DeviceMethod, _ path: String, _ body: JSON? = nil) async -> DeviceRefusal? {
    await deviceSend(method, path, body).refusal
}

struct DevicePart: Equatable, Sendable {
    let name: String
    var file: URL? = nil
    var value: String = ""
}

func devicePartJson(_ part: DevicePart) -> JSON {
    if let file = part.file { return .object(["name": .string(part.name), "file": .string(file.path)]) }
    return .object(["name": .string(part.name), "value": .string(part.value)])
}

func deviceUpload(_ method: DeviceMethod, _ path: String, _ parts: [DevicePart]) async -> DeviceAnswer {
    let sent = JSON.array(parts.map(devicePartJson))
    return await offMain { deviceAnswer(Qgc.call(DEVICE_UPLOAD, method.rawValue, path, sent)) }
}

func deviceRead<T>(_ path: String, _ parse: (JSON) -> T?) async -> T? {
    let answer: JSON? = await offMain { Qgc.call(DEVICE_READ, path) }
    guard let answer, answer["ok"].bool else { return nil }
    return answer["result"].objectOrNil.flatMap(parse)
}

func deviceQr(_ text: String) async -> Data? {
    let answer: JSON? = await offMain { Qgc.call(DEVICE_QR, text) }
    guard let answer, answer["ok"].bool else { return nil }
    let encoded = answer["result"].string
    return encoded.isEmpty ? nil : Data(base64Encoded: encoded, options: .ignoreUnknownCharacters)
}

func deviceReach(_ url: String) async -> Bool? {
    let answer: JSON? = await offMain { Qgc.call(DEVICE_REACH, url) }
    guard let answer, answer["ok"].bool else { return nil }
    return answer["result"].boolOrNil
}
