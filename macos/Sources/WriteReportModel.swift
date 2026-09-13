import Foundation

enum WriteReport {
    static func failure(_ what: String) -> String {
        "Could not change \(what). It is unchanged."
    }

    // Qt names a reason on every write it turns down, and every one of them is about the ASKING,
    // not about the vehicle: the path does not resolve, it names an object rather than a property,
    // the property is not there, it has no WRITE accessor, the value is the wrong shape for it.
    // None of them is a state the operator is in or can leave. A write refused on those terms will
    // be refused identically on the next press and every press after it, so the sentence that
    // invites a retry is the wrong one to show.
    //
    // The presence of a reason is the whole discriminator, and it is the core's own convention
    // rather than a reading of the words: a set that failed with nothing said is a failure this
    // head cannot characterise, and guessing at it would be the plausible default that absence
    // makes worse. The reason itself names a property and a C++ class, which tells the operator
    // nothing they can act on -- but it is exactly what they need to have to hand when they report
    // it, and until now it did not survive Bridge.set.
    // A write that a claimed path answers carries more than ok -- the zoom sends back what it
    // asked for, what the camera took, and whether those differ. Bridge.set hands the whole answer
    // over now, so the caller that wants the rest can have it and the ones that only want a verdict
    // read it here. nil is written; a string is the reason, empty when none came back.
    static func reason(_ answer: [String: Any]) -> String? {
        guard (answer["ok"] as? NSNumber)?.boolValue != true else { return nil }
        return (answer["reason"] as? String) ?? ""
    }

    static func refusal(_ what: String, _ reason: String) -> String {
        guard !reason.isEmpty else { return failure(what) }
        return "Could not change \(what), and this build has no way to: \(reason). It is unchanged."
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

enum EditedField {
    static func shown(typed: String, held: String, editing: Bool) -> String {
        editing ? typed : held
    }
}
