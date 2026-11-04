import AVFoundation
import SwiftUI
import UIKit

private let DEVICE_CAMERA = "deviceCamera"
private let QUARTER_TURN = 90
private let FULL_TURN = 360
private let FRONT_CAMERA = 1
// Phone camera sensors are mounted landscape: the back one a quarter turn clockwise of portrait, the front one three.
private let BACK_SENSOR_DEGREES = 90
private let FRONT_SENSOR_DEGREES = 270

func deviceCameraRotation(_ sensorDegrees: Int, front: Bool, displayDegrees: Int) -> Int {
    front ? (sensorDegrees + displayDegrees) % FULL_TURN : (sensorDegrees - displayDegrees + FULL_TURN) % FULL_TURN
}

func interfaceDegrees(_ orientation: UIInterfaceOrientation) -> Int {
    switch orientation {
    case .landscapeRight: QUARTER_TURN
    case .portraitUpsideDown: 2 * QUARTER_TURN
    case .landscapeLeft: 3 * QUARTER_TURN
    default: 0
    }
}

func deviceCameraTurn(_ camera: Int, displayDegrees: Int) -> Int {
    let front = camera == FRONT_CAMERA
    return deviceCameraRotation(front ? FRONT_SENSOR_DEGREES : BACK_SENSOR_DEGREES, front: front, displayDegrees: displayDegrees)
}

@MainActor
private func shownOrientation() -> UIInterfaceOrientation {
    UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.first?.interfaceOrientation ?? .portrait
}

private struct CameraTurn: Equatable {
    let camera: Int?
    let display: Int
}

// When the video source is this phone's own camera, asks for camera access and keeps the picture upright as the
// screen turns.
struct DeviceCameraHost: View {
    @QgcPath(VIDEO_VIEW) private var videoJson
    @State private var display = 0

    var body: some View {
        let camera = videoJson?[DEVICE_CAMERA].int
        GeometryReader { geometry in
            Color.clear.onChange(of: geometry.size, initial: true) { display = interfaceDegrees(shownOrientation()) }
        }
        .allowsHitTesting(false)
        .accessibilityHidden(true)
        .onReceive(NotificationCenter.default.publisher(for: UIDevice.orientationDidChangeNotification)) { _ in
            display = interfaceDegrees(shownOrientation())
        }
        .task(id: camera) {
            guard camera != nil, AVCaptureDevice.authorizationStatus(for: .video) == .notDetermined else { return }
            if await AVCaptureDevice.requestAccess(for: .video) { _ = await offMain { VideoCommands.restart() } }
        }
        .onChange(of: CameraTurn(camera: camera, display: display), initial: true) { _, turn in
            guard let camera = turn.camera else { return }
            let rotation = deviceCameraTurn(camera, displayDegrees: turn.display)
            offMain { VideoCommands.setDeviceCameraRotation(rotation) }
        }
    }
}
