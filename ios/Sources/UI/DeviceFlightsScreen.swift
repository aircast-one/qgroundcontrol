import SwiftUI

private let FLIGHT_CHART_HEIGHT: CGFloat = 112

struct DeviceFlightsScreen: View {
    let props: DeviceScreenProps
    @DeviceView("flights") private var reading
    @State private var lanes: [String]? = storedLanes()

    private func pickLanes(_ keys: [String]) {
        storeLanes(keys)
        lanes = keys
    }

    var body: some View {
        if let view = deviceView(reading, parseFlights) {
            DeviceText(text: "One entry per run of this device. None of it reaches you in the air — the ground station only gets the few lines worth interrupting a flight for.")
            if view.flights.isEmpty {
                DeviceNotice(notice: DeviceBadge(level: DeviceLevel.Neutral, text: "No flights recorded yet. This device starts one every time it powers up."))
            }
            ForEach(view.flights, id: \.id) { flight in
                FlightRow(flight: flight, linked: props.query["flight"] == flight.id, lanes: lanes, onLanes: pickLanes)
            }
        } else {
            FootNote(text: DEVICE_WAITING)
        }
    }
}

private struct FlightRow: View {
    let flight: FlightEntry
    let linked: Bool
    let lanes: [String]?
    let onLanes: ([String]) -> Void
    @Environment(\.theme) private var theme
    @Environment(\.LocalDeviceScroll) private var scroll
    @State private var active: Int?

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            DeviceDisclosure(flight.title, summary: flight.summary, badge: flight.badge, initiallyOpen: flight.open || linked) {
                FlightBody(flight: flight, active: active, onActive: { active = $0 }, lanes: lanes, onLanes: onLanes)
            }
            Divider().overlay(theme.colors.outlineVariant)
        }
        .id(flight.id)
        .task(id: linked) {
            guard linked else { return }
            try? await Task.sleep(for: .milliseconds(100))
            withAnimation { scroll?.scrollTo(flight.id, anchor: .top) }
        }
    }
}

private struct FlightBody: View {
    let flight: FlightEntry
    let active: Int?
    let onActive: (Int?) -> Void
    let lanes: [String]?
    let onLanes: ([String]) -> Void
    @State private var alertsOnly = false

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if hasLines(flight) {
                DeviceButtonRow {
                    DeviceChip(label: "Everything", selected: !alertsOnly) { alertsOnly = false }
                    DeviceChip(label: "Alerts only", selected: alertsOnly) { alertsOnly = true }
                }
                ForEach(Array(flight.sections.enumerated()), id: \.offset) { _, part in
                    FlightPartRows(part: part, alertsOnly: alertsOnly, active: active, onActive: onActive)
                }
            } else {
                DeviceText(text: "Nothing worth recording — the device had no faults to report.")
            }
            if let trace = flight.trace { FlightLanes(trace: trace, active: active, lanes: lanes, onLanes: onLanes) }
            if !flight.dropped.isEmpty { DeviceNotice(notice: DeviceBadge(level: DeviceLevel.Warn, text: flight.dropped)) }
            FlightFiles(flight: flight)
        }
        .padding(.bottom, 16)
    }
}

private struct FlightPartRows: View {
    let part: FlightPart
    let alertsOnly: Bool
    let active: Int?
    let onActive: (Int?) -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if !part.title.isEmpty {
                PlanFlowRow(spacing: 12, lineSpacing: 8) {
                    Text(part.title.uppercased()).font(.labelMedium).foregroundStyle(theme.colors.onSurfaceVariant)
                    if !part.span.isEmpty {
                        Text(part.span).font(deviceMono(.labelSmall)).foregroundStyle(theme.colors.onSurfaceVariant)
                    }
                }
                .padding(EdgeInsets(top: 16, leading: 8, bottom: 4, trailing: 8))
            }
            ForEach(part.videos, id: \.camera) { SortieVideoLinks(video: $0) }
            ForEach(part.lines.filter { !alertsOnly || $0.alert }, id: \.index) { line in
                LineRow(line: line, active: active == line.index, onActive: onActive)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, 8)
    }
}

private func lineColor(_ line: FlightLine, _ theme: Theme) -> Color {
    if line.heldBack { return theme.colors.onSurfaceVariant }
    return line.level == DeviceLevel.Neutral ? theme.colors.onSurface : deviceLevelColor(line.level, theme)
}

private struct LineRow: View {
    let line: FlightLine
    let active: Bool
    let onActive: (Int?) -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Button { onActive(active ? nil : line.index) } label: {
            HStack(alignment: .firstTextBaseline, spacing: 12) {
                Text(line.clock).font(deviceMono(.bodySmall)).foregroundStyle(theme.colors.onSurfaceVariant).frame(width: 64, alignment: .leading)
                VStack(alignment: .leading, spacing: 2) {
                    Text(line.text).font(.bodyMedium).foregroundStyle(lineColor(line, theme))
                    if !line.repeats.isEmpty || line.heldBack {
                        HStack(spacing: 12) {
                            if !line.repeats.isEmpty {
                                Text(line.repeats).font(deviceMono(.labelSmall)).foregroundStyle(theme.colors.onSurfaceVariant)
                            }
                            if line.heldBack {
                                Text("pilot not told")
                                    .font(.labelSmall)
                                    .foregroundStyle(theme.colors.onSurfaceVariant)
                                    .padding(.horizontal, 8)
                                    .padding(.vertical, 2)
                                    .overlay(Capsule().stroke(theme.colors.outlineVariant))
                            }
                        }
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(active ? theme.colors.surfaceContainerHighest : .clear, in: RoundedRectangle(cornerRadius: Corner.extraSmall))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }
}

private struct SortieVideoLinks: View {
    let video: SortieVideo
    @Environment(\.LocalDeviceOrigin) private var origin

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            LinkButton(href: resolveDeviceUrl(origin, video.url), text: video.label, look: .Outlined, icon: .download)
            if let share = video.share { SortieShareRow(share: share) }
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
    }
}

private struct SortieShareRow: View {
    let share: SortieShare
    @Environment(\.theme) private var theme
    @Environment(\.LocalChangeNotice) private var notice
    @QgcPath private var json: JSON?
    @State private var asking = false
    @State private var showingQr = false

    init(share: SortieShare) {
        self.share = share
        _json = QgcPath(renderPath(share))
    }

    var body: some View {
        let render = parseRender(json)
        VStack(alignment: .leading, spacing: 4) {
            if render.canStart {
                Button(share.label) { asking = true }.buttonStyle(.bordered)
            }
            if !render.progress.isEmpty {
                Text(render.progress).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
            }
            if !render.error.isEmpty {
                Text(render.error).font(.bodySmall).foregroundStyle(theme.colors.error)
            }
            if !render.url.isEmpty {
                PlanFlowRow(spacing: 4, lineSpacing: 4) {
                    Button { showingQr = true } label: { Image(.qrCode2).frame(width: Metrics.iconButton, height: Metrics.iconButton).contentShape(Rectangle()).macHover(Circle()) }
                        .buttonStyle(.plain)
                        .accessibilityLabel("Show the share link as a QR code")
                        .macTooltip("Show the share link as a QR code")
                    LinkButton(href: render.url, text: "Open the share version", look: .Text, external: true)
                    Button("Copy link") { copyToClipboard(render.url, notice) }.buttonStyle(.text)
                }
                Text(render.expiry).font(.labelSmall).foregroundStyle(theme.colors.onSurfaceVariant)
            }
        }
        .sheet(isPresented: $showingQr) { QrDialog(title: "Share version", text: render.url) { showingQr = false } }
        .deviceConfirm(ActionConfirm(title: share.label, text: share.confirm, confirm: "Send"), isPresented: $asking) {
            let share = share
            Task { _ = await requestRender(share) }
        }
    }
}

@Observable private final class LanePlot {
    var zoom: Zoom?
    var cursor: Double?
    var brush: Zoom?

    init() {}

    func down(_ at: Double) {
        brush = Zoom(from: at, to: at)
        cursor = at
    }

    func move(_ at: Double) {
        brush?.to = at
    }

    func up() {
        if let brush, abs(brush.to - brush.from) >= MIN_BRUSH { zoom = composeZoom(zoom, brush.from, brush.to) }
        brush = nil
    }
}

private struct ChartRequest: Equatable {
    let trace: JSON
    let zoom: Zoom?
}

private struct FlightLanes: View {
    let trace: JSON
    let active: Int?
    let lanes: [String]?
    let onLanes: ([String]) -> Void
    @Environment(\.theme) private var theme
    @State private var plot = LanePlot()
    @State private var chart: FlightChart?

    var body: some View {
        Group {
            if let drawn = chart { lanesView(drawn) }
        }
        .task(id: ChartRequest(trace: trace, zoom: plot.zoom)) {
            let loaded = await flightChart(trace, plot.zoom)
            if !Task.isCancelled { chart = loaded }
        }
    }

    private func lanesView(_ drawn: FlightChart) -> some View {
        let keys = laneKeys(lanes, drawn.lanes.filter(\.isDefault).map(\.key))
        let shown = drawn.lanes.filter { keys.contains($0.key) }
        return VStack(alignment: .leading, spacing: 0) {
            Divider().overlay(theme.colors.outlineVariant)
            DeviceButtonRow {
                ForEach(drawn.lanes, id: \.key) { lane in
                    LaneChip(lane: lane, selected: keys.contains(lane.key)) { onLanes(toggled(keys, lane.key)) }
                }
            }
            if !drawn.spans.isEmpty || plot.zoom != nil {
                DeviceButtonRow {
                    ForEach(drawn.spans, id: \.label) { span in
                        DeviceChip(label: span.label, selected: plot.zoom == Zoom(from: span.from, to: span.to)) { plot.zoom = Zoom(from: span.from, to: span.to) }
                    }
                    if plot.zoom != nil {
                        Button("Whole flight") { plot.zoom = nil }.buttonStyle(.text)
                    }
                }
            }
            ForEach(Array(shown.enumerated()), id: \.element.key) { index, lane in
                LaneChart(lane: lane, plot: plot, active: active, last: index == shown.count - 1)
            }
            Text(
                "Pick the lanes you want above. One cursor across them: point at any lane to read every sample at that moment. Drag across a lane to zoom them all, or jump to one sortie with the chips above. " +
                    "Dashed lines are the moments listed above, in their colour; point at a line to pick out its mark."
            )
            .font(.bodySmall)
            .foregroundStyle(theme.colors.onSurfaceVariant)
            .padding(EdgeInsets(top: 12, leading: 16, bottom: 0, trailing: 16))
        }
        .padding(.top, 8)
    }
}

private struct LaneChip: View {
    let lane: ChartLane
    let selected: Bool
    let onClick: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Button(action: onClick) {
            HStack(spacing: 6) {
                if selected { Image(.check) }
                Text(lane.chip).font(.labelLarge)
                if !lane.peak.isEmpty {
                    Text(lane.peak).font(deviceMono(.labelSmall)).foregroundStyle(theme.aircast.warning)
                }
            }
            .padding(.horizontal, 12)
            .frame(height: 32)
            .foregroundStyle(selected ? theme.colors.onSecondaryContainer : theme.colors.onSurfaceVariant)
            .background(selected ? theme.colors.secondaryContainer : .clear, in: RoundedRectangle(cornerRadius: Corner.small))
            .overlay(RoundedRectangle(cornerRadius: Corner.small).stroke(selected ? .clear : theme.colors.outline))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityAddTraits(selected ? .isSelected : [])
    }
}

private func fractionIn(_ x: CGFloat, _ width: CGFloat) -> Double { width > 0 ? Double(min(max(x / width, 0), 1)) : 0 }

private func markerColor(_ level: Int, _ theme: Theme) -> Color {
    level == DeviceLevel.Neutral ? theme.colors.onSurfaceVariant : deviceLevelColor(level, theme)
}

private struct LaneChart: View {
    let lane: ChartLane
    let plot: LanePlot
    let active: Int?
    let last: Bool
    @Environment(\.theme) private var theme

    var body: some View {
        let sample = plot.cursor.flatMap { sampleAt(lane.readings.count, $0) }
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 8) {
                Text(lane.label.uppercased()).font(.labelSmall).foregroundStyle(theme.colors.onSurfaceVariant).frame(maxWidth: .infinity, alignment: .leading)
                if let sample { Text(lane.readings[sample.index]).font(deviceMono(.labelSmall)) }
            }
            GeometryReader { box in
                ZStack(alignment: .topTrailing) {
                    canvas(sample)
                    ForEach(lane.grid, id: \.y) { line in
                        Text(line.label)
                            .font(deviceMono(.labelSmall))
                            .foregroundStyle(theme.colors.onSurfaceVariant)
                            .padding(.horizontal, 4)
                            .background(theme.colors.surface.opacity(0.8), in: RoundedRectangle(cornerRadius: Corner.extraSmall))
                            .offset(y: FLIGHT_CHART_HEIGHT * (line.y / 100) - 8)
                    }
                }
                .contentShape(Rectangle())
                .onTapGesture { at in plot.cursor = fractionIn(at.x, box.size.width) }
                .gesture(
                    DragGesture(minimumDistance: 10)
                        .onChanged { drag in
                            if plot.brush == nil {
                                guard abs(drag.translation.width) > abs(drag.translation.height) else { return }
                                plot.down(fractionIn(drag.startLocation.x, box.size.width))
                            }
                            plot.move(fractionIn(drag.location.x, box.size.width))
                        }
                        .onEnded { _ in plot.up() }
                )
            }
            .frame(height: FLIGHT_CHART_HEIGHT)
            .padding(.top, 4)
            if last {
                HStack {
                    ForEach(Array(lane.ticks.enumerated()), id: \.offset) { at, tick in
                        if at > 0 { Spacer(minLength: 0) }
                        Text(tick.label).font(deviceMono(.labelSmall)).foregroundStyle(theme.colors.onSurfaceVariant)
                    }
                }
                .padding(.top, 2)
                if !lane.span.isEmpty {
                    Text(lane.span).font(.labelSmall).foregroundStyle(theme.colors.onSurfaceVariant)
                }
            }
        }
        .padding(EdgeInsets(top: 16, leading: 16, bottom: 0, trailing: 16))
    }

    private func canvas(_ sample: FlightChartSample?) -> some View {
        let ink = theme.colors.primary
        let rule = theme.colors.onSurface
        let inks = lane.markers.map { markerColor($0.level, theme) }
        let brush = plot.brush
        return Canvas { context, size in
            let across = size.width / 100
            let down = size.height / 100
            lane.grid.forEach { line in
                var path = Path()
                path.move(to: CGPoint(x: 0, y: line.y * down))
                path.addLine(to: CGPoint(x: size.width, y: line.y * down))
                context.stroke(path, with: .color(rule.opacity(0.12)), lineWidth: 1)
            }
            zip(lane.markers, inks).forEach { marker, color in
                let picked = marker.index == active
                var path = Path()
                path.move(to: CGPoint(x: marker.x * across, y: 0))
                path.addLine(to: CGPoint(x: marker.x * across, y: size.height))
                context.stroke(path, with: .color(color.opacity(picked ? 1 : 0.7)), style: StrokeStyle(lineWidth: 1, dash: picked ? [] : [2, 2]))
            }
            lane.lines.forEach { run in
                guard let first = run.first else { return }
                var path = Path()
                path.move(to: CGPoint(x: first.x * across, y: first.y * down))
                run.dropFirst().forEach { path.addLine(to: CGPoint(x: $0.x * across, y: $0.y * down)) }
                context.stroke(path, with: .color(ink), lineWidth: 1.5)
            }
            if let brush {
                let left = CGFloat(min(brush.from, brush.to)) * size.width
                context.fill(Path(CGRect(x: left, y: 0, width: CGFloat(abs(brush.to - brush.from)) * size.width, height: size.height)), with: .color(ink.opacity(0.15)))
            }
            if let sample {
                var path = Path()
                path.move(to: CGPoint(x: sample.x * across, y: 0))
                path.addLine(to: CGPoint(x: sample.x * across, y: size.height))
                context.stroke(path, with: .color(rule.opacity(0.4)), lineWidth: 1)
            }
        }
        .accessibilityLabel("\(lane.label) over the flight")
    }
}

private struct FlightFiles: View {
    let flight: FlightEntry
    @Environment(\.theme) private var theme
    @Environment(\.LocalChangeNotice) private var notice
    @Environment(\.LocalDeviceOrigin) private var origin

    var body: some View {
        let link = flightLink(resolveDeviceUrl(origin, "/fly/"), flight.id)
        VStack(alignment: .leading, spacing: 0) {
            Divider().overlay(theme.colors.outlineVariant)
            DeviceButtonRow {
                LinkButton(href: resolveDeviceUrl(origin, flight.csv), text: "Samples as CSV", look: .Outlined)
                LinkButton(href: resolveDeviceUrl(origin, flight.json), text: "Everything as JSON", look: .Text)
                Button { copyToClipboard(link, notice, "Link copied — it opens straight on this flight.") } label: {
                    HStack(spacing: 8) {
                        Image(.link)
                        Text("Copy link to this flight")
                    }
                }
                .buttonStyle(.text)
            }
            Text(
                "The CSV is one row per sample — every lane above and more besides: signal and radio, the aircraft's height, distance, battery and satellites, video per camera, viewers, what was dropped, and the box's own load and memory. " +
                    "The JSON adds every line above."
            )
            .font(.bodySmall)
            .foregroundStyle(theme.colors.onSurfaceVariant)
            .padding(.horizontal, 16)
        }
        .padding(.top, 12)
    }
}
