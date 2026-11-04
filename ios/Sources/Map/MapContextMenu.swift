import Foundation
#if os(macOS)
import AppKit
#endif

// One row of a map's right-click menu. A row without `run` is an inert caption (the item's name, a coordinate).
struct MapMenuEntry {
    let title: String
    let enabled: Bool
    let startsGroup: Bool
    let run: (() -> Void)?

    init(_ title: String, enabled: Bool = true, startsGroup: Bool = false, run: (() -> Void)? = nil) {
        self.title = title
        self.enabled = enabled
        self.startsGroup = startsGroup
        self.run = run
    }
}

// Builds a map view's menu for the hit and point under a right-click; nil means the map keeps its older
// secondary-click behaviour (open the touch sheet, select, or add).
typealias MapContextMenu = (MapHit?, Double, Double) -> [MapMenuEntry]?

#if os(macOS)
enum PlanMenuChoice: Equatable {
    case Edit(MapHit)
    case InsertWaypointAfter(index: Int)
    case DeleteItem(index: Int)
    case RemoveVertex(MapHit)
    case DeleteFence(polygon: Int)
    case DeleteCircle(index: Int)
    case DeleteRally(index: Int)
    case Split(MapHit)
    case AddTracePoint
    case AddWaypoint
    case AddTakeoff
    case AddLanding
    case AddDrop
    case AddPattern
    case AddRally
    case AddPolygonFence
    case AddCircleFence
    case SetBreachReturn
}

struct PlanMenuLine: Equatable {
    let choice: PlanMenuChoice?
    let title: String
    var enabled = true
    var startsGroup = false
}

// What the Plan tab offers right now, read from the same state its rail and selection panel use.
struct PlanMenuOffer {
    var layer: PlanLayer
    var tracing = false
    var kinds: [MissionKind] = []
    var items: [MissionItem] = []
    var itemCount = 0
    var takeoffMissing = false
    var homeSet = true
    var rally = false
    var fence = false
    var fenceRefused = false
    var breachSet = false
    var cornerRemovable = false
}

// The selection a menu's Edit makes: a radius or rotation handle edits the item or circle it belongs to.
func planEditSelection(_ hit: MapHit) -> MapHit {
    switch hit {
    case .LoiterRadius(let index), .LoiterRotation(let index): .Waypoint(index: index)
    case .CircleRadius(let index): .Circle(index: index)
    default: hit
    }
}

// Where "Insert waypoint after" puts the new waypoint: halfway along the leg to the next routed item, as the leg's
// split handle does. Nil for the last item, which instead waits for the next click on the map.
func insertAfterPoint(_ index: Int, _ items: [MissionItem]) -> TrackPoint? {
    let placed = items.filter(\.placed)
    guard placed.contains(where: { $0.index == index && $0.routed }),
          let next = placed.filter({ $0.index > index && $0.routed }).min(by: { $0.index < $1.index }) else { return nil }
    return legSplit(placed, next.index)
}

private func caption(_ title: String) -> PlanMenuLine {
    PlanMenuLine(choice: nil, title: title, enabled: false)
}

private func grouped(_ groups: [[PlanMenuLine]]) -> [PlanMenuLine] {
    var lines: [PlanMenuLine] = []
    for group in groups where !group.isEmpty {
        for (place, line) in group.enumerated() {
            var marked = line
            marked.startsGroup = !lines.isEmpty && place == 0
            lines.append(marked)
        }
    }
    return lines
}

func planMenuLines(_ hit: MapHit?, _ offer: PlanMenuOffer) -> [PlanMenuLine] {
    guard let hit else { return blankMenuLines(offer) }
    if let index = missionItemIndex(hit) {
        let title = offer.items.first { $0.index == index }.map(itemTitle) ?? selectionTitle(hit, offer.items)
        var vertex: [PlanMenuLine] = []
        if case .SurveyVertex = hit, offer.cornerRemovable {
            vertex = [PlanMenuLine(choice: .RemoveVertex(hit), title: "Remove vertex")]
        }
        return grouped([
            [caption(title)],
            [PlanMenuLine(choice: .Edit(planEditSelection(hit)), title: "Edit")] + vertex
                + [PlanMenuLine(choice: .InsertWaypointAfter(index: index), title: "Insert waypoint after")],
            index > HOME_ITEM ? [PlanMenuLine(choice: .DeleteItem(index: index), title: "Delete")] : [],
        ])
    }
    switch hit {
    case .Midpoint(let path, _, _):
        return [PlanMenuLine(choice: .Split(hit), title: path == MISSION_SPLIT_PATH ? "Insert waypoint here" : "Add corner here")]
    case .FenceVertex(let polygon, _):
        return grouped([
            [caption(selectionTitle(hit, offer.items))],
            [PlanMenuLine(choice: .Edit(hit), title: "Edit")]
                + (offer.cornerRemovable ? [PlanMenuLine(choice: .RemoveVertex(hit), title: "Remove vertex")] : []),
            [PlanMenuLine(choice: .DeleteFence(polygon: polygon), title: "Delete fence")],
        ])
    case .ShapeCentre(_, let owner), .ShapeRadius(_, let owner):
        return grouped([
            [caption(selectionTitle(hit, offer.items))],
            [PlanMenuLine(choice: .Edit(hit), title: "Edit")],
            [PlanMenuLine(choice: .DeleteFence(polygon: owner), title: "Delete fence")],
        ])
    case .Circle(let index), .CircleCentre(let index), .CircleRadius(let index):
        return grouped([
            [caption(selectionTitle(hit, offer.items))],
            [PlanMenuLine(choice: .Edit(planEditSelection(hit)), title: "Edit")],
            [PlanMenuLine(choice: .DeleteCircle(index: index), title: "Delete circle")],
        ])
    case .Rally(let index):
        return grouped([
            [caption(selectionTitle(hit, offer.items))],
            [PlanMenuLine(choice: .Edit(hit), title: "Edit")],
            [PlanMenuLine(choice: .DeleteRally(index: index), title: "Delete rally point")],
        ])
    case .BreachReturn:
        return grouped([
            [caption(selectionTitle(hit, offer.items))],
            [PlanMenuLine(choice: .Edit(hit), title: "Edit\u{2026}")],
        ])
    default:
        return []
    }
}

private func blankMenuLines(_ offer: PlanMenuOffer) -> [PlanMenuLine] {
    if offer.tracing { return [PlanMenuLine(choice: .AddTracePoint, title: "Add trace point here")] }
    func addable(_ kind: String) -> Bool { kindAllows(offer.kinds, kind) || !offer.homeSet }
    switch offer.layer {
    case .Mission:
        let takeoff = kindOffered(offer.kinds, KIND_TAKEOFF) && offer.takeoffMissing
            ? [PlanMenuLine(choice: .AddTakeoff, title: "Add takeoff here", enabled: addable(KIND_TAKEOFF))] : []
        let drop = kindOffered(offer.kinds, KIND_DROP)
            ? [PlanMenuLine(choice: .AddDrop, title: "Add drop here", enabled: offer.itemCount > 0 && addable(KIND_DROP))] : []
        let pattern = scanPatterns(offer.kinds).isEmpty ? [] : [PlanMenuLine(choice: .AddPattern, title: "Add pattern here\u{2026}")]
        return [PlanMenuLine(choice: .AddWaypoint, title: "Add waypoint here")] + takeoff
            + [PlanMenuLine(choice: .AddLanding, title: "Add landing here", enabled: offer.itemCount > 0)] + drop + pattern
    case .Rally:
        return [PlanMenuLine(choice: .AddRally, title: "Add rally point here", enabled: offer.rally)]
    case .Fence:
        guard !offer.fenceRefused else { return [] }
        return grouped([
            [
                PlanMenuLine(choice: .AddPolygonFence, title: "Add polygon fence", enabled: offer.fence),
                PlanMenuLine(choice: .AddCircleFence, title: "Add circle fence", enabled: offer.fence),
            ],
            [PlanMenuLine(choice: .SetBreachReturn, title: offer.breachSet ? "Move breach return here" : "Set breach return here", enabled: offer.fence)],
        ])
    }
}

// Pops the entries up as a native menu at `location`, in the map view's own (AppKit) coordinates.
func showMapMenu(_ entries: [MapMenuEntry], _ view: NSView, _ location: NSPoint) {
    guard entries.contains(where: { $0.run != nil }) else { return }
    let menu = NSMenu()
    menu.autoenablesItems = false
    let target = MapMenuTarget(entries)
    for (index, entry) in entries.enumerated() {
        if entry.startsGroup && menu.numberOfItems > 0 { menu.addItem(NSMenuItem.separator()) }
        let item = NSMenuItem(title: entry.title, action: entry.run == nil ? nil : #selector(MapMenuTarget.chosen(_:)), keyEquivalent: "")
        item.tag = index
        item.target = target
        // A menu item holds its target weakly; the represented object keeps the closures alive while the menu is up.
        item.representedObject = target
        item.isEnabled = entry.enabled && entry.run != nil
        menu.addItem(item)
    }
    // Leave the gesture callback before the menu's tracking loop starts.
    DispatchQueue.main.async {
        _ = menu.popUp(positioning: nil, at: location, in: view)
    }
}

private final class MapMenuTarget: NSObject {
    private let entries: [MapMenuEntry]

    init(_ entries: [MapMenuEntry]) {
        self.entries = entries
    }

    @objc func chosen(_ item: NSMenuItem) {
        guard entries.indices.contains(item.tag) else { return }
        entries[item.tag].run?()
    }
}
#endif
