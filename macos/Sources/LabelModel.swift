import Foundation

enum LabelAnswer {
    // A bridge read that produced nothing is not an answer, and the identifier standing in for one
    // is a fallback rather than a label. Caching the fallback makes a one-off failure permanent:
    // the picker resolves about a hundred names at once, and a read during the Qt busy window on
    // window entry returns nothing for all of them. Every one would then show its raw identifier
    // for the life of the process, with no later read ever attempted.
    static func cacheable(_ value: Any?) -> String? {
        guard let read = value as? String, !read.isEmpty else { return nil }
        return read
    }
}
