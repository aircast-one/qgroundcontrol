import SwiftUI

struct PowerTile: Equatable {
    var label: String
    var value: String
    var units: String
}

private let POWER_TILE_FACTS = [("voltage", "VOLTAGE"), ("current", "CURRENT"), ("mahConsumed", "USED")]

func powerTiles(_ view: JSON?) -> [PowerTile] {
    guard let view, view["available"].bool, let facts = view["packs"][0]["facts"].arrayOrNil else { return [] }
    let byName = Dictionary(facts.filter { $0.object != nil }.map { ($0["name"].string, $0) }, uniquingKeysWith: { _, last in last })
    return POWER_TILE_FACTS.compactMap { name, label in
        byName[name].map { fact in
            PowerTile(label: label, value: fact["valueString"].string, units: fact["units"].string == "v" ? "V" : fact["units"].string)
        }
    }
}

struct PowerLiveCard: View {
    @QgcPath("view.battery") private var json
    @Environment(\.theme) private var theme

    var body: some View {
        let tiles = powerTiles(json)
        if !tiles.isEmpty {
            HStack(alignment: .top) {
                ForEach(Array(tiles.enumerated()), id: \.offset) { at, tile in
                    if at > 0 { Spacer(minLength: Space.s2) }
                    VStack(alignment: .leading, spacing: 0) {
                        Text(tile.label).font(.labelSmall).foregroundStyle(theme.colors.onSurfaceVariant)
                        HStack(alignment: .lastTextBaseline, spacing: Space.s1) {
                            Text(tile.value).font(.titleLarge)
                            Text(tile.units).font(.labelMedium).foregroundStyle(theme.colors.onSurfaceVariant)
                        }
                    }
                }
            }
            .padding(Space.s4)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(theme.colors.surfaceContainer, in: RoundedRectangle(cornerRadius: Corner.large))
            .padding(.horizontal, Space.s4)
            .padding(.vertical, Space.s2)
        }
    }
}
