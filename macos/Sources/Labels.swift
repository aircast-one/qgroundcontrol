import Foundation

enum Labels {
    private static var cache: [String: String] = [:]

    // A label is a pure function of the identifier -- label.rs declares DEPS: &[] and humanise()
    // reads no backend -- so the answer is cached rather than asked again: opening the instrument
    // picker resolves roughly a hundred of them at once.
    static func humanise(_ identifier: String) -> String {
        if let known = cache[identifier] { return known }
        guard let read = LabelAnswer.cacheable(Bridge.group("view.label(\(identifier))")["value"])
        else { return identifier }
        cache[identifier] = read
        return read
    }
}
