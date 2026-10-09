import SwiftUI
import UIKit

private let TERRAIN_COLOUR = Color(hex: 0x8D6E63)
private let PLANNED_COLOUR = Color(hex: 0x4FC3F7)
private let COLLISION_COLOUR = Color.red
private let MISSING_COLOUR = Color.yellow
private let PATTERN_COLOUR = Color.green.opacity(0.5)
private let MARKER_TAP_SLOP_PX: CGFloat = 48
private let FULL_TERRAIN = 0.98

func terrainRuns(_ profile: TerrainProfile, _ width: CGFloat, _ height: CGFloat) -> [[CGPoint]] {
    profileRuns(profile, width, height) { $0.terrain }
}

func plannedRuns(_ profile: TerrainProfile, _ width: CGFloat, _ height: CGFloat) -> [[CGPoint]] {
    profileRuns(profile, width, height) { $0.planned.isFinite ? $0.planned : nil }
}

private func profileRuns(_ profile: TerrainProfile, _ width: CGFloat, _ height: CGFloat, _ value: (ProfilePoint) -> Double?) -> [[CGPoint]] {
    let distance = profile.distance
    guard distance > 0, profile.drawable else { return [] }
    return profile.points.reduce([[CGPoint]()]) { runs, point in
        let last = runs.last ?? []
        guard let altitude = value(point) else { return last.isEmpty ? runs : runs + [[]] }
        let at = CGPoint(x: point.distance / distance * width, y: height - (altitude - profile.lowest) / profile.span * height)
        return runs.dropLast() + [last + [at]]
    }
    .filter { $0.count >= 2 }
}

func missingSpans(_ profile: TerrainProfile, _ width: CGFloat) -> [(CGFloat, CGFloat)] {
    let distance = profile.distance
    guard distance > 0 else { return [] }
    return zip(profile.points, profile.points.dropFirst())
        .filter { from, to in from.terrain == nil || to.terrain == nil }
        .map { from, to in (from.distance / distance * width, to.distance / distance * width) }
}

func collisionSegments(_ profile: TerrainProfile, _ width: CGFloat, _ height: CGFloat) -> [(CGPoint, CGPoint)] {
    let distance = profile.distance
    guard distance > 0 else { return [] }
    func at(_ point: ProfilePoint) -> CGPoint {
        CGPoint(x: point.distance / distance * width, y: height - (point.planned - profile.lowest) / profile.span * height)
    }
    return zip(profile.points, profile.points.dropFirst())
        .filter { from, to in from.collision && to.collision && from.planned.isFinite && to.planned.isFinite }
        .map { from, to in (at(from), at(to)) }
}

func markerX(_ distance: Double, _ profile: TerrainProfile, _ width: CGFloat) -> CGFloat {
    profile.distance > 0 ? distance / profile.distance * width : 0
}

func tappedSequence(_ profile: TerrainProfile, _ width: CGFloat, _ tapX: CGFloat) -> Int? {
    let candidates = profile.markers.flatMap { marker in
        ([marker.distance] + (marker.endDistance.map { [$0] } ?? [])).map { (markerX($0, profile, width), marker.sequence) }
    }
    if let nearest = candidates.min(by: { abs($0.0 - tapX) < abs($1.0 - tapX) }), abs(nearest.0 - tapX) <= MARKER_TAP_SLOP_PX {
        return nearest.1
    }
    return profile.markers.first { marker in
        marker.endDistance.map { end in markerX(marker.distance, profile, width) <= tapX && tapX <= markerX(end, profile, width) } == true
    }?.sequence
}

func groundOutline(_ terrain: [CGPoint], _ height: CGFloat) -> [CGPoint] {
    guard terrain.count >= 2, let first = terrain.first, let last = terrain.last else { return [] }
    return terrain + [CGPoint(x: last.x, y: height), CGPoint(x: first.x, y: height)]
}

private func pathOf(_ points: [CGPoint]) -> Path {
    Path { $0.addLines(points) }
}

func heightRange(_ profile: TerrainProfile) -> String {
    profile.flat ? "\(profile.lowestText) AMSL" : "\(profile.bandText) AMSL"
}

func profileLabel(_ profile: TerrainProfile) -> String {
    let coverage: String = if !profile.hasTerrain {
        " \u{00b7} ground height unknown"
    } else if profile.terrainCoverage <= 0 {
        " \u{00b7} ground height at points, none along the route"
    } else if profile.terrainCoverage < FULL_TERRAIN {
        " \u{00b7} ground height for \(max(Int(profile.terrainCoverage * 100), 1))% of the route"
    } else {
        ""
    }
    return (terrainWarning(profile.clearance).map { "\($0) " } ?? "") + heightRange(profile) + " \u{00b7} \(profile.distanceText)" + coverage
}

let ELEVATION_PROVIDER = "settings.flightMapSettings.elevationMapProvider.rawValue"
let SHOW_MISSION_ITEM_STATUS = "settings.planViewSettings.showMissionItemStatus"

func missionItemStatusShown(_ setting: JSON?) -> Bool {
    guard let setting, setting.has("value") else { return true }
    return setting["value"].bool(true)
}

func elevationCredit(_ notice: String) -> String? { notice.isBlank ? nil : "Powered by \(notice)" }

private let labelFont = UIFont.systemFont(ofSize: 11, weight: .medium)

private func measured(_ text: String) -> CGSize {
    (text as NSString).size(withAttributes: [.font: labelFont])
}

struct TerrainProfileView: View {
    let profile: TerrainProfile
    let notice: String
    var selectedSequence: Int? = nil
    var onSelect: (Int) -> Void = { _ in }
    @Environment(\.theme) private var theme
    @Environment(\.displayScale) private var displayScale

    var body: some View {
        if profile.points.isEmpty {
            EmptyView()
        } else if !profile.drawable {
            Text("Plan a route with altitudes to see a profile.")
                .font(.bodySmall)
                .foregroundStyle(theme.colors.onSurfaceVariant)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.vertical, 4)
        } else {
            VStack(spacing: 0) {
                Text(profileLabel(profile))
                    .font(.labelSmall)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, 8)
                if !profile.heightHeader.isBlank {
                    HStack {
                        Text("Elevation").font(.labelSmall).frame(maxWidth: .infinity, alignment: .leading)
                        Text(profile.heightHeader).font(.labelSmall)
                    }
                    .padding(.horizontal, 8)
                }
                if let credit = elevationCredit(notice) {
                    Text(credit).font(.labelSmall).frame(maxWidth: .infinity).multilineTextAlignment(.center)
                }
                chart
                    .frame(maxWidth: .infinity)
                    .frame(height: 110)
                    .padding(.bottom, 16)
                    .background(theme.colors.surface.opacity(0.88))
            }
        }
    }

    private var chart: some View {
        let margin = (profile.heightTicks.map { measured($0).width }.max() ?? 0) + 8
        let top: CGFloat = 8
        let ink = theme.colors.onSurface
        let accent = theme.colors.primary
        let onAccent = theme.colors.onPrimary
        let paper = theme.colors.surface
        return GeometryReader { geometry in
            let width = max(geometry.size.width - margin - 8, 1)
            let height = max(geometry.size.height - top - 16, 1)
            Canvas { context, _ in
                context.translateBy(x: margin, y: top)
                terrainRuns(profile, width, height).forEach { run in
                    context.fill(pathOf(groundOutline(run, height)), with: .color(TERRAIN_COLOUR.opacity(0.45)))
                    context.stroke(pathOf(run), with: .color(TERRAIN_COLOUR), lineWidth: 1.5)
                }
                missingSpans(profile, width).forEach { from, to in
                    context.stroke(pathOf([CGPoint(x: from, y: height), CGPoint(x: to, y: height)]), with: .color(MISSING_COLOUR), lineWidth: 4)
                }
                profile.heightTicks.enumerated().forEach { index, tick in
                    let y = height - height * CGFloat(index) / CGFloat(max(profile.heightTicks.count - 1, 1))
                    context.stroke(pathOf([CGPoint(x: 0, y: y), CGPoint(x: width, y: y)]), with: .color(ink.opacity(0.3)), lineWidth: 0.5)
                    let size = measured(tick)
                    context.draw(Text(tick).font(TypeScale.labelSmall.font).foregroundColor(ink), at: CGPoint(x: -size.width - 4, y: y - size.height / 2), anchor: .topLeading)
                }
                profile.distanceTicks.enumerated().forEach { index, tick in
                    let size = measured(tick)
                    let x = width * CGFloat(index) / CGFloat(max(profile.distanceTicks.count - 1, 1))
                    let left = min(max(x - size.width / 2, 0), max(width - size.width, 0))
                    context.draw(Text(tick).font(TypeScale.labelSmall.font).foregroundColor(ink), at: CGPoint(x: left, y: height), anchor: .topLeading)
                }
                plannedRuns(profile, width, height).forEach { run in
                    context.stroke(pathOf(run), with: .color(PLANNED_COLOUR), lineWidth: 1.5)
                }
                collisionSegments(profile, width, height).forEach { from, to in
                    context.stroke(pathOf([from, to]), with: .color(COLLISION_COLOUR), lineWidth: 4)
                }
                profile.markers.forEach { marker in
                    let start = markerX(marker.distance, profile, width)
                    let ends = marker.endDistance.map { [(markerX($0, profile, width), "\(marker.lastSequence ?? marker.sequence)")] } ?? []
                    if let end = marker.endDistance {
                        let band = measured(marker.pattern)
                        let right = markerX(end, profile, width)
                        context.fill(Path(CGRect(x: start, y: height - band.height, width: right - start, height: band.height)), with: .color(PATTERN_COLOUR))
                        context.draw(Text(marker.pattern).font(TypeScale.labelSmall.font).foregroundColor(ink), at: CGPoint(x: (start + right - band.width) / 2, y: height - band.height), anchor: .topLeading)
                    }
                    ([(start, marker.label)] + ends).forEach { x, text in
                        context.stroke(pathOf([CGPoint(x: x, y: 0), CGPoint(x: x, y: height)]), with: .color(ink), lineWidth: 0.5)
                        let size = measured(text)
                        let radius = max(size.width, size.height) / 2 + 3
                        let centre = CGPoint(x: x, y: height - radius)
                        let chosen = marker.sequence == selectedSequence
                        context.fill(Path(ellipseIn: CGRect(x: centre.x - radius, y: centre.y - radius, width: radius * 2, height: radius * 2)), with: .color(chosen ? accent : ink.opacity(0.8)))
                        context.draw(Text(text).font(TypeScale.labelSmall.font).foregroundColor(chosen ? onAccent : paper), at: centre, anchor: .center)
                    }
                }
            }
            .contentShape(Rectangle())
            .onTapGesture { location in
                tappedSequence(profile, width * displayScale, (location.x - margin) * displayScale).map(onSelect)
            }
        }
    }
}
