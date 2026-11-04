import Foundation

enum DeviceLink {
    static func connect(_ address: String) async -> String? {
        await offMain { refusal(Qgc.call(DEVICE_CONNECT, address), subject: "The device") }
    }

    static func disconnect() {
        offMain { _ = Qgc.invoke(DEVICE_DISCONNECT) }
    }
}
