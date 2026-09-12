import Foundation

struct VehicleLink: Equatable {
    let name: String
    let primary: Bool
    let heard: Bool?

    init(_ json: [String: Any]) {
        name = (json["name"] as? String) ?? ""
        primary = (json["primary"] as? NSNumber)?.boolValue ?? false
        heard = (json["commLost"] as? NSNumber).map { !$0.boolValue }
    }
}

enum VehicleLinks {
    static let silent = "No contact"
    static let hearing = "Receiving"
    static let unwatched = "Not watched"

    static func list(_ json: Any?) -> [VehicleLink] {
        ((json as? [Any]) ?? []).compactMap { $0 as? [String: Any] }
            .map(VehicleLink.init)
            .filter { !$0.name.isEmpty }
    }

    static func status(_ link: VehicleLink) -> String {
        guard let heard = link.heard else { return unwatched }
        return heard ? hearing : silent
    }

    static func label(_ link: VehicleLink, among links: [VehicleLink]) -> String {
        guard link.primary, links.count > 1 else { return link.name }
        return "\(link.name) (primary)"
    }

    static func rows(_ links: [VehicleLink]) -> [DetailRow] {
        guard links.count > 1 || links.contains(where: { $0.heard == false }) else { return [] }
        return links.map { DetailRow(label: label($0, among: links), value: status($0)) }
    }
}
