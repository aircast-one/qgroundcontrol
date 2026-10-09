import Foundation

let VEHICLE_LINKS = "view.vehicleLinks"

struct VehicleLink: Equatable {
    let commLost: Bool?
}

struct LossFailsafe: Equatable {
    let action: String
    let after: Double?
}

struct VehicleLinks: Equatable {
    let available: Bool
    let links: [VehicleLink]
    var contactLost: Bool? = nil
    var failsafe: LossFailsafe? = nil
}

func vehicleLinks(_ view: JSON?) -> VehicleLinks? {
    guard let view, view["class"].string == "VehicleLinks" else { return nil }
    return VehicleLinks(
        available: view["available"].bool,
        links: view["links"].objects.map { link in
            VehicleLink(commLost: link["commLost"].boolOrNil)
        },
        contactLost: view["contactLost"].boolOrNil,
        failsafe: view["lossAction"].string.isEmpty ? nil : LossFailsafe(action: view["lossAction"].string, after: view["lossAfter"].isNull ? nil : view["lossAfter"].double(.nan))
    )
}

struct LinkCell: Equatable {
    let text: String
    let degraded: Bool
}

func linkCell(_ links: VehicleLinks?) -> LinkCell? {
    guard let links, links.available, links.links.count >= 2 else { return nil }
    let silent = links.links.filter { $0.commLost == true }
    switch silent.count {
    case 0: return LinkCell(text: "\(links.links.count) links", degraded: false)
    case links.links.count: return LinkCell(text: "no link heard", degraded: true)
    case 1: return LinkCell(text: "1 link lost", degraded: true)
    default: return LinkCell(text: "\(silent.count) links lost", degraded: true)
    }
}

func linkNames(_ view: JSON?) -> [String] {
    (view?["links"].objects ?? []).map { $0["name"].string }
}
