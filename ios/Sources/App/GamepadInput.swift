import Foundation
import GameController
import os

let JOYSTICK_DEVICES = "joystick.devices"
let JOYSTICK_INPUT = "joystick.input"
private let SAMPLE_MS: Int64 = 2
private let IDLE_MS: Int64 = 250

func streams(_ view: JSON?) -> Bool {
    guard let view, view.object != nil else { return false }
    return (view["vehicle"].bool && view["enabled"].bool) || !view["calibration"].isNull
}

private let AXIS_SCALE: Float = 32767
private let HAT_UP = 0x01
private let HAT_RIGHT = 0x02
private let HAT_DOWN = 0x04
private let HAT_LEFT = 0x08
private let HAT_THRESHOLD: Float = 0.5

let GAMEPAD_BUTTONS: [(GCExtendedGamepad) -> GCControllerButtonInput?] = [
    { $0.buttonA }, { $0.buttonB }, { $0.buttonX }, { $0.buttonY },
    { $0.leftShoulder }, { $0.rightShoulder }, { $0.leftTrigger }, { $0.rightTrigger },
    { $0.buttonOptions }, { $0.buttonMenu }, { $0.buttonHome },
    { $0.leftThumbstickButton }, { $0.rightThumbstickButton },
    { $0.controller?.physicalInputProfile.buttons[GCInputXboxPaddleOne] },
    { $0.controller?.physicalInputProfile.buttons[GCInputXboxPaddleTwo] },
    { $0.controller?.physicalInputProfile.buttons[GCInputXboxPaddleThree] },
    { $0.controller?.physicalInputProfile.buttons[GCInputXboxPaddleFour] },
    { _ in nil }, { _ in nil }, { _ in nil }, { _ in nil },
]

private let GAMEPAD_AXES: [(GCExtendedGamepad) -> Float] = [
    { $0.leftThumbstick.xAxis.value }, { -$0.leftThumbstick.yAxis.value },
    { $0.rightThumbstick.xAxis.value }, { -$0.rightThumbstick.yAxis.value },
    { $0.leftTrigger.value }, { $0.rightTrigger.value },
]

func powerStateText(_ status: Int) -> String {
    switch GCDeviceBattery.State(rawValue: status) {
    case .charging: "Charging"
    case .discharging: "Discharging"
    case .full: "Full"
    default: ""
    }
}

private func deviceDetails(_ controller: GCController) -> [String: Any] {
    let battery = controller.battery.flatMap { $0.batteryLevel.isNaN ? nil : $0 }
    return [
        "vendorId": 0,
        "productId": 0,
        "guid": "",
        "playerIndex": controller.playerIndex.rawValue,
        "batteryPercent": battery.map { Int(($0.batteryLevel * 100).rounded()) } ?? -1,
        "powerState": battery.map { powerStateText($0.batteryState.rawValue) } ?? "",
        "rumble": controller.haptics != nil,
        "led": controller.light != nil,
        "gyroscope": controller.motion?.hasRotationRate == true,
        "accelerometer": controller.motion != nil,
    ]
}

func hatBits(_ x: Float, _ y: Float) -> Int {
    (y < -HAT_THRESHOLD ? HAT_UP : 0) | (y > HAT_THRESHOLD ? HAT_DOWN : 0) |
        (x < -HAT_THRESHOLD ? HAT_LEFT : 0) | (x > HAT_THRESHOLD ? HAT_RIGHT : 0)
}

func scaled(_ value: Float) -> Int { Int((min(max(value, -1), 1) * AXIS_SCALE).rounded()) }

private struct Pad: Equatable {
    let name: String
    let values: [Float]
    let pressed: [Bool]
    let hat: Int
}

private func sample(_ name: String, _ gamepad: GCExtendedGamepad) -> Pad {
    Pad(
        name: name,
        values: GAMEPAD_AXES.map { $0(gamepad) },
        pressed: GAMEPAD_BUTTONS.map { $0(gamepad)?.isPressed ?? false },
        hat: hatBits(gamepad.dpad.xAxis.value, -gamepad.dpad.yAxis.value)
    )
}

private struct Sampled: Equatable {
    let name: String
    let axes: [Int]
    let buttons: [Bool]
    let hat: Int
}

private func controllerName(_ controller: GCController) -> String {
    controller.vendorName ?? controller.productCategory
}

enum GamepadInput {
    private static let pads = OSAllocatedUnfairLock(initialState: [ObjectIdentifier: Pad]())
    private static var sampler: Task<Void, Never>?
    private static var watching: [NSObjectProtocol] = []

    @MainActor
    static func start() {
        watching = [Notification.Name.GCControllerDidConnect, .GCControllerDidDisconnect].map { name in
            NotificationCenter.default.addObserver(forName: name, object: nil, queue: .main) { _ in MainActor.assumeIsolated { rescan() } }
        }
        rescan()
        sampler?.cancel()
        sampler = Task.detached(priority: .userInitiated) {
            var sent: [Sampled] = []
            var streaming = false
            var checkedAt: Int64 = 0
            while !Task.isCancelled {
                let sampled = pads.withLock { $0.values.sorted { $0.name < $1.name } }
                    .map { Sampled(name: $0.name, axes: $0.values.map(scaled), buttons: $0.pressed, hat: $0.hat) }
                let now = Int64(Date().timeIntervalSince1970 * 1000)
                if !sampled.isEmpty && now - checkedAt >= IDLE_MS {
                    streaming = streams(Qgc.get(JOYSTICK_VIEW))
                    checkedAt = now
                }
                if streaming || sampled != sent {
                    sampled.forEach { pad in Qgc.invoke(JOYSTICK_INPUT, pad.name, pad.axes, pad.buttons, [pad.hat]) }
                    sent = sampled
                }
                try? await Task.sleep(for: .milliseconds(sampled.isEmpty ? IDLE_MS : SAMPLE_MS))
            }
        }
    }

    @MainActor
    static func stop() {
        sampler?.cancel()
        sampler = nil
        watching.forEach(NotificationCenter.default.removeObserver)
        watching = []
    }

    @MainActor
    private static func rescan() {
        let found = GCController.controllers().filter { $0.extendedGamepad != nil }
        found.forEach { controller in
            let id = ObjectIdentifier(controller)
            let name = controllerName(controller)
            controller.extendedGamepad?.valueChangedHandler = { gamepad, _ in
                let pad = sample(name, gamepad)
                pads.withLock { $0[id] = pad }
            }
        }
        let present = Dictionary(uniqueKeysWithValues: found.compactMap { controller in
            controller.extendedGamepad.map { (ObjectIdentifier(controller), sample(controllerName(controller), $0)) }
        })
        pads.withLock { known in known = present.merging(known.filter { present[$0.key] != nil }) { _, held in held } }
        let devices = JSON(found.map { controller in
            deviceDetails(controller).merging([
                "name": controllerName(controller),
                "axes": GAMEPAD_AXES.count,
                "buttons": GAMEPAD_BUTTONS.count,
                "hats": 1,
                "gamepad": true,
            ]) { _, named in named }
        })
        offMain { Qgc.invoke(JOYSTICK_DEVICES, devices) }
    }
}
