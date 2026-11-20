import Foundation

private let vehicleArmed = "vehicle.armed"
private let vehicleFlightMode = "vehicle.flightMode"
private let vehicleChecklistState = "vehicle.checkListState"
private let vehicleVtolForwardFlight = "vehicle.vtolInFwdFlight"
private let vehicleHeading = "vehicle.heading"
private let vehicleCoordinate = "vehicle.coordinate"
private let vehicleTakeoff = "vehicle.guidedModeTakeoff"
private let vehicleLand = "vehicle.guidedModeLand"
private let vehicleReturnToLaunch = "vehicle.guidedModeRTL"
private let vehicleChangeAltitude = "vehicle.guidedModeChangeAltitude"
private let vehiclePause = "vehicle.pauseVehicle"
private let vehicleStartMission = "vehicle.startMission"
private let vehicleAbortLanding = "vehicle.abortLanding"
private let vehicleGripper = "vehicle.sendGripperAction"
private let vehicleFixPayload = "vehicle.fixPayload"
private let vehicleStopRoi = "vehicle.stopGuidedModeROI"
private let vehicleForceArm = "vehicle.forceArm"
private let vehicleEmergencyStop = "vehicle.emergencyStop"
private let vehicleMotorTest = "vehicle.motorTest"
private let vehicleRcOverride = "vehicle.setRcChannelOverride"
private let vehicleRcRelease = "vehicle.releaseRcChannelOverride"
private let vehicleRcClear = "vehicle.clearRcChannelOverrides"
private let vehicleRemoteIdEmergency = "vehicle.remoteIDManager.setEmergency"
private let vehicleResetMessages = "vehicle.resetAllMessages"
private let vehicleClearMessages = "vehicle.clearMessages"

enum VehicleCommands {
    @discardableResult static func setArmed(_ armed: Bool) -> Bool { Qgc.set(vehicleArmed, armed) }
    @discardableResult static func setFlightMode(_ mode: String) -> Bool { Qgc.set(vehicleFlightMode, mode) }
    @discardableResult static func setChecklistState(_ state: Int) -> Bool { Qgc.set(vehicleChecklistState, state) }

    static func heading() -> JSON { Qgc.get(vehicleHeading) }
    static func coordinate() -> JSON { Qgc.get(vehicleCoordinate) }

    static func takeoffRefusal() -> String? { Qgc.refusalOf(vehicleTakeoff) }
    static func takeoffRefusal(_ meters: Double) -> String? { Qgc.refusalOf(vehicleTakeoff, meters) }
    static func landRefusal() -> String? { Qgc.refusalOf(vehicleLand) }
    @discardableResult static func returnToLaunch(_ smart: Bool) -> Bool { Qgc.invoke(vehicleReturnToLaunch, smart) }
    static func returnToLaunchRefusal(_ smart: Bool) -> String? { Qgc.refusalOf(vehicleReturnToLaunch, smart) }
    static func pauseRefusal() -> String? { Qgc.refusalOf(vehiclePause) }
    static func changeSpeedRefusal(_ command: String, _ metersSecond: Double) -> String? { Qgc.refusalOf("vehicle.\(command)", metersSecond) }
    static func changeAltitudeRefusal(_ deltaMeters: Double, pause: Bool) -> String? { Qgc.refusalOf(vehicleChangeAltitude, deltaMeters, pause) }
    static func startMissionRefusal() -> String? { Qgc.refusalOf(vehicleStartMission) }
    static func abortLandingRefusal(_ climbMeters: Double) -> String? { Qgc.refusalOf(vehicleAbortLanding, climbMeters) }
    static func gripperRefusal(_ action: Int, bay: Int = 0) -> String? { Qgc.refusalOf(vehicleGripper, action, bay) }
    static func fixPayloadRefusal(_ code: String) -> String? { Qgc.refusalOf(vehicleFixPayload, code) }
    static func stopRoiRefusal() -> String? { Qgc.refusalOf(vehicleStopRoi) }
    static func forceArmRefusal() -> String? { Qgc.refusalOf(vehicleForceArm) }
    static func forwardFlightRefusal(_ forward: Bool) -> String? { Qgc.writeRefusal(vehicleVtolForwardFlight, forward) }
    @discardableResult static func emergencyStop() -> Bool { Qgc.invoke(vehicleEmergencyStop) }
    @discardableResult static func motorTest(_ motor: Int, percent: Int, seconds: Int, inOrder: Bool) -> Bool { Qgc.invoke(vehicleMotorTest, motor, percent, seconds, inOrder) }
    @discardableResult static func overrideRcChannel(_ channel: Int, pwm: Int) -> Bool { Qgc.invoke(vehicleRcOverride, channel, pwm) }
    @discardableResult static func releaseRcChannel(_ channel: Int) -> Bool { Qgc.invoke(vehicleRcRelease, channel) }
    @discardableResult static func clearRcOverrides() -> Bool { Qgc.invoke(vehicleRcClear) }
    @discardableResult static func declareRemoteIdEmergency(_ declare: Bool) -> Bool { Qgc.invoke(vehicleRemoteIdEmergency, declare) }
    @discardableResult static func resetAllMessages() -> Bool { Qgc.invoke(vehicleResetMessages) }
    @discardableResult static func clearMessages() -> Bool { Qgc.invoke(vehicleClearMessages) }
}
