import Foundation

enum WriteReport {
    static func failure(_ what: String) -> String {
        "Could not change \(what). It is unchanged."
    }
}
