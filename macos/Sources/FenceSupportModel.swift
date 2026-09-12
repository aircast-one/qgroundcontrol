import Foundation

struct FenceSupport: Equatable {
    let read: Bool
    let supported: Bool

    static let unread = FenceSupport(read: false, supported: false)

    static func answered(_ supported: Bool) -> FenceSupport {
        FenceSupport(read: true, supported: supported)
    }

    init(read: Bool, supported: Bool) {
        self.read = read
        self.supported = supported
    }

    init(answer: Any?) {
        guard let answered = (answer as? NSNumber)?.boolValue else {
            self.init(read: false, supported: false)
            return
        }
        self.init(read: true, supported: answered)
    }

    var offers: Bool { read && supported }

    var refusal: String? {
        guard read else { return FenceSupport.unreadRefusal }
        return supported ? nil : FenceSupport.unsupportedRefusal }

    static let unreadRefusal = "The plan has not been read yet."
    static let unsupportedRefusal = "This vehicle does not accept a geofence."
}
