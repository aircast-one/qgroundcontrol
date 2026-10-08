import SwiftUI

let AIRCAST_LINK_VIEW = "view.aircastLink"
private let SIGNAL_MAXIMUM = 100.0

struct AircastLink: Equatable {
    let qualityText: String
    let signalText: String
    let network: String
    let modem: String
    let bitrateText: String
    let qualityHistory: [Double]
    let bitrateHistory: [Double]
}

private func numbers(_ array: [JSON]?) -> [Double] { (array ?? []).map { $0.double ?? .nan } }

func aircastLink(_ view: JSON?) -> AircastLink? {
    guard let it = view, it["shown"].bool else { return nil }
    return AircastLink(
        qualityText: it["qualityText"].string,
        signalText: it["signalText"].string,
        network: it["network"].string,
        modem: it["modem"].string,
        bitrateText: it["bitrateText"].string,
        qualityHistory: numbers(it["qualityHistory"].arrayOrNil),
        bitrateHistory: numbers(it["bitrateHistory"].arrayOrNil)
    )
}

func sparkRuns(_ values: [Double], maximum: Double) -> [[(Float, Float)]] {
    guard values.count >= 2 else { return [] }
    let largest = maximum > 0 ? maximum : (values.max() ?? 0)
    let peak = largest > 0 ? largest : 1.0
    let step = Float(1) / Float(values.count - 1)
    return values.enumerated()
        .reduce([[(Float, Float)]()]) { runs, entry in
            let (index, value) = entry
            guard !(value < 0) else { return runs.last?.isEmpty == true ? runs : runs + [[]] }
            return runs.dropLast() + [runs.last! + [(Float(index) * step, Float(1 - min(value, peak) / peak))]]
        }
        .filter { $0.count >= 2 }
}

private struct Sparkline: View {
    @Environment(\.theme) private var theme
    let values: [Double]
    let maximum: Double

    var body: some View {
        Canvas { context, size in
            sparkRuns(values, maximum: maximum).forEach { run in
                let path = Path { path in
                    path.addLines(run.map { CGPoint(x: CGFloat($0.0) * size.width, y: CGFloat($0.1) * (size.height - 1) + 0.5) })
                }
                context.stroke(path, with: .color(theme.aircast.success), lineWidth: 1.5)
            }
        }
        .frame(maxWidth: .infinity)
        .frame(height: 48)
    }
}

struct AircastLinkCell: View {
    @Environment(\.theme) private var theme
    @QgcPath(AIRCAST_LINK_VIEW) private var json
    @State private var open = false

    var body: some View {
        if let link = aircastLink(json) {
            Text("\(link.qualityText) \(link.bitrateText)")
                .font(.labelMedium)
                .foregroundStyle(theme.colors.onSurfaceVariant)
                .lineLimit(1)
                .onTapGesture { open = true }
                .background {
                    if open {
                        AircastSheet(onDismissRequest: { open = false }) {
                            VStack(alignment: .leading, spacing: 6) {
                                Text("Cellular link").font(.titleSmall)
                                ForEach([("Signal:", link.signalText), ("Network:", link.network), ("Modem:", link.modem), ("Video bitrate:", link.bitrateText)], id: \.0) { label, value in
                                    Text("\(label)  \(value)").font(.bodySmall)
                                }
                                Text("Signal, up to the last hour").font(.labelSmall)
                                Sparkline(values: link.qualityHistory, maximum: SIGNAL_MAXIMUM)
                                Text("Video bitrate, up to the last hour").font(.labelSmall)
                                Sparkline(values: link.bitrateHistory, maximum: 0)
                            }
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(.horizontal, Space.s5)
                            .padding(.bottom, Space.s6)
                        }
                    }
                }
        }
    }
}
