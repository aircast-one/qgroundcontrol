import SwiftUI

func hostNoticesPath(_ acknowledgedThrough: Int64) -> String { "view.hostNotices(\(acknowledgedThrough))" }

struct NoticeBatch: Equatable {
    var through: Int64
    var destination: String?
    var banners: [String]
    var unknownKinds: [String]
    var errorBanners: [String]
    var dialogs: [AppMessage] = []
}

struct AppMessage: Equatable, Hashable, Identifiable {
    var title: String
    var text: String
    var action: String? = nil

    var id: Self { self }
}

let REBOOT_VEHICLE_ACTION = "rebootVehicle"
let OPEN_SETUP_ACTION = "openSetup"

func noticeBatch(_ view: JSON?) -> NoticeBatch? {
    guard let view, let unseen = view["unseen"].arrayOrNil else { return nil }
    let notices = unseen.filter { $0.object != nil && ($0["id"].int64 ?? -1) >= 0 }
    guard let through = notices.compactMap({ $0["id"].int64 }).max() else { return nil }
    let destination = view["destination"].string
    return NoticeBatch(
        through: through,
        destination: destination.isBlank ? nil : destination,
        banners: view["banners"].array.map(\.string).filter { !$0.isBlank },
        unknownKinds: notices.filter { !$0["known"].bool(true) }.map { $0["kind"].string },
        errorBanners: notices.filter { $0["kind"].string == VEHICLE_ERROR_KIND }.map { $0["banner"].string }.filter { !$0.isBlank }.distinct(),
        dialogs: view["dialogs"].objects.map { dialog in
            let action = dialog["action"].string
            return AppMessage(title: dialog["title"].string, text: dialog["text"].string, action: action.isBlank ? nil : action)
        }
    )
}

let REPEAT_QUIET_MS: Int64 = 30_000
let VEHICLE_ERROR_KIND = "vehicleError"
let RESET_ERROR_LEVEL_MESSAGES = "vehicle.resetErrorLevelMessages"

let ADDITIONAL_ERRORS = "Additional errors received"

func criticalBanner(_ errors: [String]) -> String? {
    errors.first.map { first in errors.count > 1 ? "\(first) \u{00b7} \(ADDITIONAL_ERRORS)" : first }
}

struct AppMessageDialog: View {
    let message: AppMessage
    var onOpenSetup: () -> Void = {}
    let onDismiss: () -> Void

    var body: some View {
        Color.clear
            .invisibleAnchor()
            .alert(message.title, isPresented: .constant(true)) {
                Button(message.action == OPEN_SETUP_ACTION ? "Open Setup" : "OK") {
                    if message.action == REBOOT_VEHICLE_ACTION { offMain { Qgc.invoke(REBOOT_VEHICLE) } }
                    if message.action == OPEN_SETUP_ACTION { onOpenSetup() }
                    onDismiss()
                }
                if message.action == REBOOT_VEHICLE_ACTION { Button("Cancel", role: .cancel, action: onDismiss) }
                if message.action == OPEN_SETUP_ACTION { Button("Later", role: .cancel, action: onDismiss) }
            } message: {
                Text(message.text)
            }
    }
}

func quietBanners(_ banners: [String], _ shownAt: [String: Int64], _ now: Int64) -> [String] {
    banners.distinct().filter { banner in shownAt[banner].map { now - $0 < REPEAT_QUIET_MS } != true }
}

