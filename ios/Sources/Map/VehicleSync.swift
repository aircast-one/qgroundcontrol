import Combine
import Foundation

enum VehicleSync { case Offline, Busy, Ready }

func vehicleSyncState(_ offline: Bool, _ syncing: Bool) -> VehicleSync {
    offline ? .Offline : syncing ? .Busy : .Ready
}

func syncRefusal(_ state: VehicleSync, _ action: String) -> String? {
    switch state {
    case .Offline: "No vehicle to \(action)"
    case .Busy: "Already syncing, wait for it to finish"
    case .Ready: nil
    }
}

struct UploadGate: Equatable {
    var canSend: Bool
    var canProceed: Bool
    var pausesFirst: Bool
    var heading: String
    var refusal: String
    var proceedTitle: String
}

private func readiness(_ view: JSON?) -> JSON? {
    guard let readiness = view?["readiness"], readiness.object != nil, !readiness["ready"].bool else { return nil }
    return readiness
}

func notReadyToSend(_ view: JSON?) -> String? {
    readiness(view).map { $0["reason"].string.ifBlank("The plan is not ready to send.") }
}

let NEWEST_PATTERN = -1

enum PlanFocus {
    static let requests = CurrentValueSubject<Int?, Never>(nil)
    static let newPattern = CurrentValueSubject<Int?, Never>(nil)

    static func notReady(_ view: JSON?) {
        if let next = nextNotReady(view) { requests.value = next }
    }
}

func nextNotReady(_ view: JSON?) -> Int? {
    readiness(view).map { $0["next"].int(-1) }.flatMap { $0 > 0 ? $0 : nil }
}

func uploadGate(_ view: JSON?) -> UploadGate? {
    guard let upload = view?["upload"], upload.object != nil else { return nil }
    return UploadGate(
        canSend: upload["canSend"].bool,
        canProceed: upload["canProceed"].bool,
        pausesFirst: upload["pausesFirst"].bool,
        heading: upload["heading"].string,
        refusal: upload["refusal"].string,
        proceedTitle: upload["proceedTitle"].string
    )
}

enum UploadStep: Equatable {
    case Send
    case Confirm(UploadGate)
    case Refuse(String)
}

func uploadStep(_ gate: UploadGate?, notReady: String? = nil) -> UploadStep {
    if let notReady { return .Refuse(notReady) }
    guard let gate else { return .Refuse("The plan could not be checked against the vehicle.") }
    if gate.canSend { return .Send }
    if gate.canProceed { return .Confirm(gate) }
    return .Refuse(gate.refusal.ifBlank("This plan cannot be uploaded."))
}

struct PlanSupport: Equatable {
    var fence: Bool
    var rally: Bool
    var fenceRefused: Bool = false
    var rallyRefused: Bool = false
}

func planSupport(_ view: JSON?) -> PlanSupport {
    let actions = view?["actions"]
    return PlanSupport(
        fence: actions?["addFence"].bool == true,
        rally: actions?["addRally"].bool == true,
        fenceRefused: view?["fenceSupported"] == .bool(false),
        rallyRefused: view?["rallySupported"] == .bool(false)
    )
}

func freshPlanView() -> JSON? {
    let view = Qgc.get("view.plan")
    return view.object != nil ? view : nil
}

let UPLOADED = "Uploaded"

func uploadLabel(_ offline: Bool, _ syncing: Bool, _ dirty: Bool, _ hasItems: Bool) -> String {
    syncing ? "Uploading…" : !dirty && hasItems && !offline ? UPLOADED : "Upload"
}
