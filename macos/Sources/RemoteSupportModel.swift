import Foundation

struct RemoteSupport: Equatable {
    let host: String
    let forwarding: Bool

    static let empty = RemoteSupport(host: "", forwarding: false)

    var canConnect: Bool { !forwarding && !host.isEmpty }

    var status: String {
        forwarding
            ? "MAVLink is being forwarded. It keeps going until the app restarts."
            : "Nothing is being forwarded."
    }

    var note: String {
        "Sends this vehicle's MAVLink traffic to the support host so someone there can watch the flight with you."
    }
}
