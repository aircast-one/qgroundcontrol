import Foundation

enum WriteReport {
    static func failure(_ what: String) -> String {
        "Could not change \(what). It is unchanged."
    }
}

enum PlanFile {
    static func notSaved(_ name: String) -> String {
        "Could not save the plan to \(name). Nothing was written."
    }

    static func notLoaded(_ name: String) -> String {
        "Could not read a plan from \(name). The plan you had is unchanged."
    }
}
