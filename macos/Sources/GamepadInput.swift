import Foundation
import GameController

enum GamepadLayout {
    static let axisScale: Float = 32767
    static let hatUp = 0x01
    static let hatRight = 0x02
    static let hatDown = 0x04
    static let hatLeft = 0x08
    static let hatThreshold: Float = 0.5

    static func scaled(_ value: Float) -> Int {
        Int((min(max(value, -1), 1) * axisScale).rounded())
    }

    static func hatBits(x: Float, y: Float) -> Int {
        (y > hatThreshold ? hatUp : 0) | (y < -hatThreshold ? hatDown : 0)
            | (x < -hatThreshold ? hatLeft : 0) | (x > hatThreshold ? hatRight : 0)
    }

    static func axes(_ pad: GCExtendedGamepad) -> [Int] {
        [pad.leftThumbstick.xAxis.value, -pad.leftThumbstick.yAxis.value,
         pad.rightThumbstick.xAxis.value, -pad.rightThumbstick.yAxis.value,
         pad.leftTrigger.value, pad.rightTrigger.value].map(scaled)
    }

    static func buttons(_ pad: GCExtendedGamepad) -> [Bool] {
        let listed: [GCControllerButtonInput?] = [
            pad.buttonA, pad.buttonB, pad.buttonX, pad.buttonY,
            pad.leftShoulder, pad.rightShoulder, pad.leftTrigger, pad.rightTrigger,
            pad.buttonOptions, pad.buttonMenu, pad.buttonHome,
            pad.leftThumbstickButton, pad.rightThumbstickButton,
        ]
        return listed.map { $0?.isPressed ?? false }
    }

    static let axisCount = 6
    static let buttonCount = 13
}

final class GamepadInput {
    static let shared = GamepadInput()

    private var sampler: Timer?
    private var observers: [NSObjectProtocol] = []

    func start() {
        guard sampler == nil else { return }
        let center = NotificationCenter.default
        observers = [NSNotification.Name.GCControllerDidConnect, .GCControllerDidDisconnect].map { name in
            center.addObserver(forName: name, object: nil, queue: .main) { [weak self] _ in self?.report() }
        }
        GCController.shouldMonitorBackgroundEvents = true
        report()
        sampler = Timer.scheduledTimer(withTimeInterval: 0.02, repeats: true) { [weak self] _ in self?.sample() }
    }

    private var pads: [(String, GCExtendedGamepad)] {
        GCController.controllers().compactMap { controller in
            controller.extendedGamepad.map { (controller.vendorName ?? "Gamepad", $0) }
        }
    }

    private func report() {
        let devices: [[String: Any]] = pads.map { name, _ in
            ["name": name, "axes": GamepadLayout.axisCount, "buttons": GamepadLayout.buttonCount, "hats": 1, "gamepad": true]
        }
        Bridge.invoke("joystick.devices", [devices])
    }

    private func sample() {
        pads.forEach { name, pad in
            let hat = GamepadLayout.hatBits(x: pad.dpad.xAxis.value, y: pad.dpad.yAxis.value)
            Bridge.invoke("joystick.input", [name, GamepadLayout.axes(pad), GamepadLayout.buttons(pad), [hat]])
        }
    }
}
