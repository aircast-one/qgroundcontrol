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

    static func from(_ elements: [Any]) -> [MissionCommand] {
        elements.compactMap(MissionCommand.init(json:))
            .reduce(into: [MissionCommand]()) { unique, command in
                if !unique.contains(where: { $0.command == command.command }) {
                    unique.append(command)
                }
            }
    }
}
