import SwiftUI

// Stand-ins for the Device tab pieces that are not ported yet. Each block is replaced by the real port
// of the Android file named in its MARK; delete the block (or this whole file) when that port lands.

// MARK: - DeviceCamerasScreen.kt, DeviceCameraSetupScreen.kt, DeviceCameraWizard.kt, CameraWizard.kt

struct DeviceCamerasScreen: View {
    let props: DeviceScreenProps

    var body: some View { FootNote(text: "Camera settings are not available in this app yet.") }
}

struct DeviceCameraSetupScreen: View {
    let props: DeviceScreenProps

    var body: some View { FootNote(text: "Camera settings are not available in this app yet.") }
}

struct CameraWizardView: Equatable {
    let name: String
    let isNew: Bool
    let json: JSON
}

func parseCameraWizard(_ json: JSON?) -> CameraWizardView? {
    guard let json else { return nil }
    return CameraWizardView(name: json["name"].string, isNew: json["new"].bool, json: json)
}

struct CameraWizard: View {
    let view: CameraWizardView
    var scan: Bool = false
    var onCancel: (() -> Void)? = nil
    var onStep: (Int) -> Void = { _ in }
    let onDone: (String) -> Void

    var body: some View { FootNote(text: "Setting up a camera is not available in this app yet.") }
}
