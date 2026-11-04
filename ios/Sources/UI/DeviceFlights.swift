import Foundation
import SwiftUI

struct FlightLine: Equatable {
    let index: Int
    let clock: String
    let text: String
    let level: Int
    let alert: Bool
    let repeats: String
    let heldBack: Bool
}

struct SortieShare: Equatable {
    let label: String
    let confirm: String
    let camera: String
    let from: String
    let to: String
}

struct SortieVideo: Equatable {
    let camera: String
    let label: String
    let url: String
    let share: SortieShare?
}

struct FlightPart: Equatable {
    let title: String
    let span: String
    let videos: [SortieVideo]
    let lines: [FlightLine]
}

struct FlightEntry: Equatable {
    let id: String
    let title: String
    let summary: String
    let badge: DeviceBadge
    let open: Bool
    let sections: [FlightPart]
    let trace: JSON?
    let dropped: String
    let csv: String
    let json: String
}

struct FlightsView: Equatable {
    let flights: [FlightEntry]
}

private func parseShare(_ json: JSON?) -> SortieShare? {
    json.map { SortieShare(label: $0["label"].string, confirm: $0["confirm"].string, camera: $0["camera"].string, from: $0["from"].string, to: $0["to"].string) }
}

private func parsePart(_ json: JSON) -> FlightPart {
    FlightPart(
        title: json["title"].string,
        span: json["span"].string,
        videos: json["videos"].objects.map {
            SortieVideo(camera: $0["camera"].string, label: $0["label"].string, url: $0["url"].string, share: parseShare($0["share"].objectOrNil))
        },
        lines: json["lines"].objects.map {
            FlightLine(
                index: $0["index"].int(0),
                clock: $0["clock"].string,
                text: $0["text"].string,
                level: $0["level"].int(DeviceLevel.Neutral),
                alert: $0["alert"].bool,
                repeats: $0["repeats"].string,
                heldBack: $0["heldBack"].bool
            )
        }
    )
}

private func parseFlight(_ json: JSON) -> FlightEntry {
    FlightEntry(
        id: json["id"].string,
        title: json["title"].string,
        summary: json["summary"].string,
        badge: parseBadge(json["badge"].objectOrNil),
        open: json["open"].bool,
        sections: json["sections"].objects.map(parsePart),
        trace: json["trace"].objectOrNil,
        dropped: json["dropped"].string,
        csv: json["csv"].string,
        json: json["json"].string
    )
}

func parseFlights(_ json: JSON) -> FlightsView? {
    json["flights"].arrayOrNil.map { flights in FlightsView(flights: flights.filter { $0.object != nil }.map(parseFlight)) }
}

func hasLines(_ flight: FlightEntry) -> Bool { flight.sections.contains { !$0.lines.isEmpty } }

func flightLink(_ base: String, _ id: String) -> String { "\(base)#device=\(uriComponent("/flights?flight=\(id)"))" }

let LANES_KEY = "flight-lanes"

func laneKeys(_ saved: [String]?, _ defaults: [String]) -> [String] { saved ?? defaults }

func toggled(_ keys: [String], _ key: String) -> [String] { keys.contains(key) ? keys.filter { $0 != key } : keys + [key] }

func storedLanes() -> [String]? { readStored(LANES_KEY).map { $0.split(separator: ",").map(String.init).filter { !$0.isEmpty } } }

func storeLanes(_ keys: [String]) { writeStored(LANES_KEY, keys.joined(separator: ",")) }

struct Zoom: Equatable {
    var from: Double
    var to: Double
}

func composeZoom(_ current: Zoom?, _ a: Double, _ b: Double) -> Zoom {
    let window = current ?? Zoom(from: 0, to: 1)
    let span = window.to - window.from
    return Zoom(from: window.from + min(a, b) * span, to: window.from + max(a, b) * span)
}

let MIN_BRUSH = 0.02

struct ChartGrid: Equatable {
    let y: CGFloat
    let label: String
}

struct ChartTick: Equatable {
    let x: CGFloat
    let label: String
}

struct ChartMarker: Equatable {
    let x: CGFloat
    let label: String
    let level: Int
    let index: Int
}

struct ChartLane: Equatable {
    let key: String
    let label: String
    let chip: String
    let peak: String
    let isDefault: Bool
    let lines: [[CGPoint]]
    let grid: [ChartGrid]
    let markers: [ChartMarker]
    let readings: [String]
    let ticks: [ChartTick]
    let span: String
}

struct ChartSpan: Equatable {
    let label: String
    let from: Double
    let to: Double
}

struct FlightChart: Equatable {
    let lanes: [ChartLane]
    let spans: [ChartSpan]
}

private let FLIGHT_CHART = "deviceFlight.chart"

private func plotUnit(_ json: JSON) -> CGFloat { CGFloat(json.double ?? .nan) }

private func parseChartLane(_ json: JSON) -> ChartLane {
    ChartLane(
        key: json["key"].string,
        label: json["label"].string,
        chip: json["chip"].string,
        peak: json["peak"].string,
        isDefault: json["default"].bool,
        lines: json["lines"].array.compactMap(\.arrayOrNil).map { run in
            run.filter { $0.arrayOrNil != nil }.map { CGPoint(x: plotUnit($0[0]), y: plotUnit($0[1])) }
        },
        grid: json["grid"].objects.map { ChartGrid(y: plotUnit($0["y"]), label: $0["label"].string) },
        markers: json["markers"].objects.map {
            ChartMarker(x: plotUnit($0["x"]), label: $0["label"].string, level: $0["level"].int(DeviceLevel.Neutral), index: $0["index"].int(0))
        },
        readings: json["readings"].strings,
        ticks: json["ticks"].objects.map { ChartTick(x: plotUnit($0["x"]), label: $0["label"].string) },
        span: json["span"].string
    )
}

func parseFlightChart(_ json: JSON?) -> FlightChart? {
    guard let json else { return nil }
    return FlightChart(
        lanes: json["lanes"].objects.map(parseChartLane),
        spans: json["spans"].objects.map { ChartSpan(label: $0["label"].string, from: $0["from"].double(0), to: $0["to"].double(1)) }
    )
}

func flightChart(_ trace: JSON, _ zoom: Zoom?) async -> FlightChart? {
    let from = zoom?.from, to = zoom?.to
    let answer: JSON? = await offMain { Qgc.call(FLIGHT_CHART, trace, from, to) }
    return parseFlightChart(answer?["result"].objectOrNil)
}

struct FlightChartSample: Equatable {
    let index: Int
    let x: CGFloat
}

func sampleAt(_ count: Int, _ fraction: Double) -> FlightChartSample? {
    guard count > 0 else { return nil }
    let index = min(max(Int((fraction * Double(count - 1)).rounded()), 0), count - 1)
    return FlightChartSample(index: index, x: count > 1 ? CGFloat(index) / CGFloat(count - 1) * 100 : 0)
}

struct FlightRender: Equatable {
    let canStart: Bool
    let progress: String
    let error: String
    let url: String
    let expiry: String
}

private let FLIGHT_RENDER = "deviceFlight.render"

func renderPath(_ share: SortieShare) -> String { "view.deviceRender(\(share.camera),\(share.from),\(share.to))" }

func parseRender(_ json: JSON?) -> FlightRender {
    FlightRender(
        canStart: json?["canStart"].bool(true) ?? true,
        progress: json?["progress"].string ?? "",
        error: json?["error"].string ?? "",
        url: json?["url"].string ?? "",
        expiry: json?["expiry"].string ?? ""
    )
}

func requestRender(_ share: SortieShare) async -> Bool {
    let camera = share.camera, from = share.from, to = share.to
    return await offMain { Qgc.invoke(FLIGHT_RENDER, camera, from, to) }
}
