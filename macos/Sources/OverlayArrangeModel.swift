import Foundation

enum OverlayArrange {
    static let maxRememberedSizes = 8

    static func key(width: Double, height: Double) -> String {
        "\(Int(width.rounded()))x\(Int(height.rounded()))"
    }

    static func clamp(_ value: Double, extent: Double, size: Double, margin: Double) -> Double {
        let low = margin
        let high = max(low, size - margin - extent)
        return max(low, min(value, high))
    }

    // A drop in the far half is measured from the far edge, so a panel put near the right
    // stays a whole number of steps from the right rather than from the left.
    static func snap(_ value: Double, extent: Double, size: Double,
                     margin: Double, grid: Double) -> Double {
        let low = margin
        let high = max(low, size - margin - extent)
        guard grid > 0 else { return max(low, min(value, high)) }
        let snapped = value + extent / 2 > size / 2
            ? high - ((high - value) / grid).rounded() * grid
            : low + ((value - low) / grid).rounded() * grid
        return max(low, min(snapped, high))
    }

    static func settles(dropped: Double, base: Double, extent: Double, threshold: Double) -> Bool {
        abs(dropped - base) < max(threshold, extent / 2)
    }

    static func remember(_ sizes: [String], _ key: String) -> [String] {
        let kept = sizes.filter { $0 != key } + [key]
        return Array(kept.suffix(maxRememberedSizes))
    }

    static func distance(from key: String, width: Double, height: Double) -> Double? {
        let parts = key.split(separator: "x", omittingEmptySubsequences: false)
        guard parts.count == 2,
              let storedWidth = Double(parts[0]), let storedHeight = Double(parts[1]),
              storedWidth.isFinite, storedHeight.isFinite else { return nil }
        return abs(storedWidth - width) + abs(storedHeight - height)
    }

    static func usable(_ sizes: [String], width: Double, height: Double) -> [String] {
        sizes.filter { distance(from: $0, width: width, height: height) != nil }
    }

    // An arrangement made at one window size is a better starting point than none at all,
    // so the nearest remembered size is used when this one has never been arranged.
    static func storedKey(_ sizes: [String], width: Double, height: Double) -> String? {
        let exact = key(width: width, height: height)
        if sizes.contains(exact) { return exact }
        return usable(sizes, width: width, height: height).min {
            (distance(from: $0, width: width, height: height) ?? .infinity)
                < (distance(from: $1, width: width, height: height) ?? .infinity)
        }
    }
}
