import Foundation

struct MissionCommand: Identifiable, Equatable {
    let command: Int
    let name: String
    let category: String
    let summary: String

    var id: Int { command }

    init?(json: Any?) {
        guard let object = json as? [String: Any],
              let command = (object["command"] as? NSNumber)?.intValue,
              let name = object["friendlyName"] as? String, !name.isEmpty else { return nil }
        self.command = command
        self.name = name
        category = (object["category"] as? String) ?? ""
        summary = (object["description"] as? String) ?? ""
    }

    // The picker's empty state was a pair of literals in PlanWindow, which swift-checks does not
    // compile. The disconnected half comes from the shared prompt rather than a fourth spelling of
    // it: VehicleSetupText.absent says "Connect a vehicle to set this up.", which is wrong here --
    // choosing a command is not setting anything up -- so this is connectPrompt's subject form.
    //
    // The connected half names the CATEGORY rather than the plan: the list is filtered by the
    // chosen category AND by what the vehicle accepts, so "no commands" without "this category"
    // would read as a vehicle that accepts nothing at all.
    static func emptyText(connected: Bool) -> String {
        connected
            ? "This category has no commands this vehicle accepts."
            : VehicleSetupText.connectPrompt(for: "commands")
    }

    static func from(_ elements: [Any]) -> [MissionCommand] {
        elements.compactMap(MissionCommand.init(json:))
            .reduce(into: [MissionCommand]()) { unique, command in
                if !unique.contains(where: { $0.command == command.command }) {
                    unique.append(command)
                }
            }
    }
}
