import Foundation

let DEFAULT_PLAN_NAME = "mission.plan"
let DEFAULT_KML_NAME = "mission.kml"
let DEFAULT_BOUNDARY_EXT = "kml"
let PLAN_EXTENSION = "plan"
let KML_EXTENSION = "kml"

func withExtension(_ name: String, _ extension: String) -> String {
    name.lowercased().hasSuffix(".\(`extension`.lowercased())") ? name : "\(name).\(`extension`)"
}

func boundaryCacheName(_ displayName: String?) -> String {
    let ext = displayName.flatMap { name in name.lastIndex(of: ".").map { name[name.index(after: $0)...].lowercased() } }
    return "boundary.\(ext.flatMap { $0.isBlank ? nil : $0 } ?? DEFAULT_BOUNDARY_EXT)"
}

let FILE_NOT_OPENED = "That file could not be opened."
let FILE_NOT_READ = "That file could not be read."

func userCancelled(_ error: Error) -> Bool { (error as? CocoaError)?.code == .userCancelled }

func mainBoundaryName(_ names: [String]) -> String? {
    names.first { $0.lowercased().hasSuffix(".shp") } ?? names.first { $0.lowercased().hasSuffix(".kml") } ?? names.first
}

func importedNothing(_ distance: Double?) -> Bool { distance.map { $0 <= 0 } ?? true }

struct PlanActions: Equatable {
    let open: Bool
    let save: Bool
    let exportKml: Bool
    let newPlan: Bool
    let clearFromVehicle: Bool
    var download: Bool = false
}

func planActions(_ view: JSON?) -> PlanActions {
    let allowed = { (name: String) in view?["actions"][name].bool == true }
    return PlanActions(
        open: allowed("open"),
        save: allowed("save"),
        exportKml: allowed("exportKml"),
        newPlan: allowed("newPlan"),
        clearFromVehicle: allowed("clearMission"),
        download: allowed("download")
    )
}

struct PlanHistory: Equatable {
    let canUndo: Bool
    let canRedo: Bool
}

func planHistory(_ view: JSON?) -> PlanHistory {
    PlanHistory(canUndo: view?["canUndo"].bool == true, canRedo: view?["canRedo"].bool == true)
}

func planIsDirty(_ view: JSON?) -> Bool { view?["dirty"].bool == true }

func planIsSyncing(_ view: JSON?) -> Bool {
    view?["sync"]["state"].string == "busy"
}

func planSyncProgress(_ view: JSON?) -> Double {
    min(max(view?["sync"]["progress"].double(0) ?? 0, 0), 1)
}

func planContainsItems(_ view: JSON?) -> Bool { view?["containsItems"].bool == true }

func saveBlockedReason(_ view: JSON?) -> String? {
    guard let readiness = view?["readiness"], readiness.object != nil else { return "The plan could not be checked for saving." }
    return readiness["ready"].bool ? nil : readiness["reason"].string.ifBlank("The plan could not be checked for saving.")
}

enum PlanConfirm: Equatable { case Open, NewPlan, ClearMission, Download }

struct ConfirmCopy: Equatable {
    let title: String
    let body: String
    let confirm: String
    var destructive: Bool = false
}

func confirmCopy(_ kind: PlanConfirm) -> ConfirmCopy {
    switch kind {
    case .Open:
        ConfirmCopy(
            title: "Plan overwrite",
            body: "You have unsaved/unsent changes. Loading from a file will lose these changes. Are you sure you want to load from a file?",
            confirm: "Load from file"
        )
    case .NewPlan:
        ConfirmCopy(
            title: "Create Plan",
            body: "Are you sure you want to remove current plan and create a new plan?",
            confirm: "Create plan"
        )
    case .Download:
        ConfirmCopy(
            title: "Plan overwrite",
            body: "You have unsaved/unsent changes. Loading from the Vehicle will lose these changes. Are you sure you want to load from the Vehicle?",
            confirm: "Load from vehicle"
        )
    case .ClearMission:
        ConfirmCopy(
            title: "Clear the mission from the vehicle?",
            body: "This removes the mission from the aircraft as well as from this plan. It cannot be undone.",
            confirm: "Clear mission",
            destructive: true
        )
    }
}

func planTitle(_ documentName: String?) -> String {
    guard let name = documentName, !name.isBlank else { return "Untitled Plan" }
    guard let dot = name.lastIndex(of: "."), dot > name.startIndex else { return name }
    return String(name[..<dot])
}

func planStatusText(_ view: JSON?) -> String { view?["status"].string ?? "" }

let DRAWN_KINDS: Set<String> = [
    "settings", "takeoff", "land", "waypoint", "command", "altitude", "roi",
    "survey", "corridor", "structure",
]

func undrawnItemNames(_ items: [JSON]) -> [String] {
    items.filter { $0.object != nil }
        .filter { !DRAWN_KINDS.contains($0["kind"].string) }
        .map { $0["name"].string }
        .filter { !$0.isBlank }
        .reduce([String]()) { kept, name in kept.contains(name) ? kept : kept + [name] }
}

func undrawnItemsWarning(_ names: [String]) -> String? {
    names.isEmpty ? nil : "The map cannot draw \(names.joined(separator: ", ")). Those items are still in the plan and will still be flown."
}
