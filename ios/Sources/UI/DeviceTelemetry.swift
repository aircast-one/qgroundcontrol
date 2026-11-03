import SwiftUI

struct DeviceEndpoint: Equatable {
    let proto: String
    let label: String
    let url: String
    let hint: String
    let note: String
    let appLink: String
    let down: Bool
    let primary: Bool
}

struct TelemetryGimbal: Equatable {
    let title: String
    let status: DeviceBadge
    let about: String
    let vendor: String
    let model: String
    let cameraAddr: String
    let mavlinkAddr: String
    let vendors: [DeviceOption]
    let models: [DeviceOption]
}

struct TelemetryCloud: Equatable {
    let status: DeviceBadge
    let text: String
}

struct TelemetrySettings: Equatable, Hashable {
    let endpoints: String
    let baud: String
}

struct TelemetryView: Equatable {
    let status: DeviceBadge
    let notices: [DeviceBadge]
    let endpoints: [DeviceEndpoint]
    let endpointNotice: DeviceBadge?
    let cloud: TelemetryCloud?
    let settings: TelemetrySettings
    let schemes: JSON
    let gimbal: TelemetryGimbal?
}

func parseEndpointList(_ json: JSON?, _ key: String) -> [DeviceEndpoint] {
    (json?[key].objects ?? []).map {
        DeviceEndpoint(
            proto: $0["proto"].string,
            label: $0["label"].string,
            url: $0["url"].string,
            hint: $0["hint"].string,
            note: $0["note"].string,
            appLink: $0["appLink"].string,
            down: $0["down"].bool,
            primary: $0["primary"].bool
        )
    }
}

private func parseGimbal(_ json: JSON?) -> TelemetryGimbal? {
    guard let json else { return nil }
    return TelemetryGimbal(
        title: json["title"].string,
        status: parseBadge(json["status"].objectOrNil),
        about: json["about"].string,
        vendor: json["vendor"].string,
        model: json["model"].string,
        cameraAddr: json["cameraAddr"].string,
        mavlinkAddr: json["mavlinkAddr"].string,
        vendors: parseOptions(json, "vendors"),
        models: parseOptions(json, "models")
    )
}

func parseTelemetry(_ json: JSON) -> TelemetryView? {
    guard let status = json["status"].objectOrNil else { return nil }
    let settings = json["settings"].objectOrNil
    return TelemetryView(
        status: parseBadge(status),
        notices: parseBadges(json, "notices"),
        endpoints: parseEndpointList(json, "endpoints"),
        endpointNotice: json["endpointNotice"].objectOrNil.map { parseBadge($0) },
        cloud: json["cloud"].objectOrNil.map { TelemetryCloud(status: parseBadge($0["status"].objectOrNil), text: $0["text"].string) },
        settings: TelemetrySettings(endpoints: settings?["endpoints"].string ?? "", baud: settings?["baud"].string ?? ""),
        schemes: json["schemes"].arrayOrNil.map { JSON.array($0) } ?? JSON.array([]),
        gimbal: parseGimbal(json["gimbal"].objectOrNil)
    )
}

struct EndpointRow: Equatable, Hashable {
    var scheme: String
    var host: String
    var port: String
}

struct EndpointOption: Equatable {
    let value: String
    let label: String
    let row: EndpointRow
}

struct EndpointRowView: Equatable {
    let row: EndpointRow
    let hostLabel: String
    let placeholder: String
    let note: String
    let problem: String
    let options: [EndpointOption]
}

struct EndpointEditor: Equatable {
    let rows: [EndpointRowView]
    let blank: EndpointRowView
    let endpoints: String
}

let TELEMETRY_ENDPOINTS = "deviceTelemetry.endpoints"

private func parseRow(_ json: JSON?) -> EndpointRow {
    EndpointRow(scheme: json?["scheme"].string ?? "", host: json?["host"].string ?? "", port: json?["port"].string ?? "")
}

private func parseRowView(_ json: JSON?) -> EndpointRowView {
    EndpointRowView(
        row: parseRow(json),
        hostLabel: json?["hostLabel"].string ?? "",
        placeholder: json?["placeholder"].string ?? "",
        note: json?["note"].string ?? "",
        problem: json?["problem"].string ?? "",
        options: (json?["options"].objects ?? []).map { EndpointOption(value: $0["value"].string, label: $0["label"].string, row: parseRow($0["row"].objectOrNil)) }
    )
}

func parseEndpointEditor(_ answer: JSON?) -> EndpointEditor? {
    guard let answer, answer["ok"].bool, let result = answer["result"].objectOrNil else { return nil }
    return EndpointEditor(rows: result["rows"].objects.map { parseRowView($0) }, blank: parseRowView(result["blank"].objectOrNil), endpoints: result["endpoints"].string)
}

private func rowsJson(_ rows: [EndpointRow]) -> JSON {
    .array(rows.map { .object(["scheme": .string($0.scheme), "host": .string($0.host), "port": .string($0.port)]) })
}

func endpointEditor(_ schemes: JSON, _ rows: [EndpointRow]?, _ stored: String) async -> EndpointEditor? {
    let spec = rows.map(rowsJson) ?? JSON.string(stored)
    let answer: JSON? = await offMain { Qgc.call(TELEMETRY_ENDPOINTS, schemes, spec) }
    return parseEndpointEditor(answer)
}

func endpointLabel(_ endpoint: DeviceEndpoint) -> String {
    !endpoint.proto.isEmpty && !endpoint.label.hasPrefix(endpoint.proto) ? "\(endpoint.proto) · \(endpoint.label)" : endpoint.label
}

struct EndpointRows: View {
    let endpoints: [DeviceEndpoint]

    var body: some View {
        ForEach(endpoints, id: \.url) { endpoint in
            CopyRow(
                label: endpointLabel(endpoint),
                text: endpoint.url,
                hint: [endpoint.hint, endpoint.note].filter { !$0.isEmpty }.joined(separator: " "),
                qr: true,
                level: endpoint.down ? DeviceLevel.Warn : DeviceLevel.Neutral
            )
        }
    }
}
