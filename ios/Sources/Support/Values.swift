import Foundation

func uptimeMillis() -> Int64 { Int64(ProcessInfo.processInfo.systemUptime * 1000) }

func saturatingInt(_ value: Double) -> Int {
    value.isNaN ? 0 : Int(exactly: value.rounded(.towardZero)) ?? (value > 0 ? .max : .min)
}

func withChanges<T>(_ value: T, _ change: (inout T) -> Void) -> T {
    var copy = value
    change(&copy)
    return copy
}

func sameDouble<T: FloatingPoint>(_ a: T?, _ b: T?) -> Bool { a == b || (a?.isNaN == true && b?.isNaN == true) }

extension Array where Element: Hashable {
    func distinct() -> [Element] {
        let first = Dictionary(enumerated().map { ($0.element, $0.offset) }, uniquingKeysWith: Swift.min)
        return enumerated().filter { first[$0.element] == $0.offset }.map(\.element)
    }
}

extension Comparable {
    func clamped(to range: ClosedRange<Self>) -> Self { min(max(self, range.lowerBound), range.upperBound) }
}

struct Keys<each T: Equatable>: Equatable {
    let values: (repeat each T)

    init(_ values: repeat each T) {
        self.values = (repeat each values)
    }

    static func == (lhs: Self, rhs: Self) -> Bool {
        for (left, right) in repeat (each lhs.values, each rhs.values) {
            guard left == right else { return false }
        }
        return true
    }
}
