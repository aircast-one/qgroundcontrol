import Foundation

struct PlanActions: Equatable {
    let open: Bool
    let save: Bool
    let exportKml: Bool
    let newPlan: Bool
    let clearFromVehicle: Bool
    let addFence: Bool
    let addRally: Bool

    static let none = PlanActions(open: false, save: false, exportKml: false, newPlan: false,
                                  clearFromVehicle: false, addFence: false, addRally: false)

    init(open: Bool, save: Bool, exportKml: Bool, newPlan: Bool,
         clearFromVehicle: Bool, addFence: Bool, addRally: Bool) {
        self.open = open
        self.save = save
        self.exportKml = exportKml
        self.newPlan = newPlan
        self.clearFromVehicle = clearFromVehicle
        self.addFence = addFence
        self.addRally = addRally
    }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any] else { return nil }
        func flag(_ name: String) -> Bool { (json[name] as? NSNumber)?.boolValue ?? false }
        open = flag("open")
        save = flag("save")
        exportKml = flag("exportKml")
        newPlan = flag("newPlan")
        clearFromVehicle = flag("clearMission")
        addFence = flag("addFence")
        addRally = flag("addRally")
    }
}
