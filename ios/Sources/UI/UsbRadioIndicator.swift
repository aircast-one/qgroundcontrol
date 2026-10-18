import SwiftUI

let USB_RADIO_VIEW = "view.usbRadio"

enum UsbRadioState: String, CaseIterable {
    case live, paused, standby, ready, stopped
}

enum UsbRadioTone {
    case error, warning
}

struct UsbRadioBadge: Equatable {
    let text: String
    let state: UsbRadioState
}

func usbRadioBadge(_ view: JSON?) -> UsbRadioBadge? {
    guard let it = view?["indicator"], it.object != nil, let state = UsbRadioState(rawValue: it["state"].string) else { return nil }
    return UsbRadioBadge(text: it["text"].string, state: state)
}

func usbRadioTone(_ state: UsbRadioState) -> UsbRadioTone? {
    switch state {
    case .stopped: return .error
    case .paused: return .warning
    default: return nil
    }
}
