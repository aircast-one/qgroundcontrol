import Foundation

struct MapScaleBar: Equatable {
    let text: String
    let fraction: Double

    static let none = MapScaleBar(text: "", fraction: 0)

    static let metres: [Double] = [5, 10, 25, 50, 100, 150, 250, 500, 1000, 2000, 5000,
                                   10000, 20000, 50000, 100000, 200000, 500000,
                                   1000000, 2000000]
    static let feet: [Double] = [10, 25, 50, 100, 250, 500, 1000, 2000, 3000, 4000, 5280,
                                 5280 * 2, 5280 * 5, 5280 * 10, 5280 * 25, 5280 * 50,
                                 5280 * 100, 5280 * 250, 5280 * 500, 5280 * 1000]

    static let footPerMetre = 3.2808399

    static func snapped(_ measured: Double, to steps: [Double]) -> (value: Double, ratio: Double)? {
        guard measured > 0, measured.isFinite, let last = steps.last else { return nil }
        for index in 0..<(steps.count - 1) where measured < (steps[index] + steps[index + 1]) / 2 {
            return (steps[index], steps[index] / measured)
        }
        return (last, last / measured)
    }

    static func metricText(_ metres: Double) -> String {
        let whole = metres.rounded()
        guard whole > 1000 else { return "\(Int(whole)) m" }
        guard whole <= 100000 else { return "\(Int((whole / 1000).rounded())) km" }
        return "\((whole / 100).rounded() / 10) km"
    }

    static func imperialText(_ feet: Double) -> String {
        let whole = feet.rounded()
        guard whole >= 5280 else { return "\(Int(whole)) ft" }
        let miles = Int((whole / 5280).rounded())
        return "\(miles) mile\(miles == 1 ? "" : "s")"
    }

    static func bar(metresAcross: Double, imperial: Bool) -> MapScaleBar {
        if imperial {
            guard let snap = snapped(metresAcross * footPerMetre, to: feet) else { return .none }
            return MapScaleBar(text: imperialText(snap.value), fraction: snap.ratio)
        }
        guard let snap = snapped(metresAcross, to: metres) else { return .none }
        return MapScaleBar(text: metricText(snap.value), fraction: snap.ratio)
    }
}
