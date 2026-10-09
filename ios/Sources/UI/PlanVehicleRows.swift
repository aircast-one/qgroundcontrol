import SwiftUI

private let APP_SETTINGS = "settings.appSettings"
private let OFFLINE_CLASSES = [("offlineEditingFirmwareClass", "Firmware"), ("offlineEditingVehicleClass", "Vehicle")]

func choosesPlanVehicle(_ connected: Bool, _ plan: JSON?) -> Bool {
    !connected && plan?["hasMissionItems"].bool != true
}

private func offlineClassFacts() -> [(String, Fact)] {
    OFFLINE_CLASSES.compactMap { name, label in
        Qgc.get("\(APP_SETTINGS).\(name)").object.map { read in
            (label, Qgc.fact(APP_SETTINGS, .object(read.merging(["name": .string(name)]) { $1 })))
        }
    }
}

struct PlanVehicleRows: View {
    @QgcPath("view.plan") private var plan
    @HasVehicle private var connected
    @State private var reloads = 0
    @State private var facts: [(String, Fact)] = []

    var body: some View {
        let choosing = choosesPlanVehicle(connected, plan)
        VStack(alignment: .leading, spacing: 0) {
            if choosing {
                ForEach(facts, id: \.1.path) { label, fact in
                    FactRow(fact: fact, title: label, onWrite: { reloads += 1 })
                }
            } else if let vehicle = plan?["planningFor"], vehicle.object != nil {
                ForEach([("Firmware", vehicle["firmware"].string), ("Vehicle", vehicle["type"].string)].filter { !$0.1.isBlank }, id: \.0) { label, value in
                    HStack {
                        Text(label).font(.bodyMedium).frame(maxWidth: .infinity, alignment: .leading)
                        Text(value).font(.bodyMedium)
                    }
                    .padding(.horizontal, Space.s5)
                    .padding(.vertical, Space.s1)
                }
            }
        }
        .task(id: "\(choosing)|\(reloads)") {
            let wanted = choosing
            facts = await offMain { wanted ? offlineClassFacts() : [] }
        }
    }
}
