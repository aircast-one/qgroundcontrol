import Foundation

enum Labels {
    private static var cache: [String: String] = [:]

    // A label is a pure function of the identifier, so the answer is cached rather than
    // asked again: opening the instrument picker resolves roughly a hundred of them at once.
    static func humanise(_ identifier: String) -> String {
        if let known = cache[identifier] { return known }
        let read = (Bridge.group("view.label(\(identifier))")["value"] as? String) ?? identifier
        cache[identifier] = read
        return read
    }
}
