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
private let vehicleStartMission = "vehicle.startMission"
private let vehicleAbortLanding = "vehicle.abortLanding"
private let vehicleGripper = "vehicle.sendGripperAction"
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
    @discardableResult static func setForwardFlight(_ forward: Bool) -> Bool { Qgc.set(vehicleVtolForwardFlight, forward) }

    static func heading() -> JSON { Qgc.get(vehicleHeading) }
    static func coordinate() -> JSON { Qgc.get(vehicleCoordinate) }

    @discardableResult static func takeoff() -> Bool { Qgc.invoke(vehicleTakeoff) }
    @discardableResult static func takeoff(_ meters: Double) -> Bool { Qgc.invoke(vehicleTakeoff, meters) }
    static func takeoffRefusal() -> String? { Qgc.refusalOf(vehicleTakeoff) }
    static func takeoffRefusal(_ meters: Double) -> String? { Qgc.refusalOf(vehicleTakeoff, meters) }
    @discardableResult static func land() -> Bool { Qgc.invoke(vehicleLand) }
    static func landRefusal() -> String? { Qgc.refusalOf(vehicleLand) }
    @discardableResult static func returnToLaunch(_ smart: Bool) -> Bool { Qgc.invoke(vehicleReturnToLaunch, smart) }
    @discardableResult static func changeSpeed(_ command: String, _ metersSecond: Double) -> Bool { Qgc.invoke("vehicle.\(command)", metersSecond) }
    @discardableResult static func changeAltitude(_ deltaMeters: Double, pause: Bool) -> Bool { Qgc.invoke(vehicleChangeAltitude, deltaMeters, pause) }
    @discardableResult static func startMission() -> Bool { Qgc.invoke(vehicleStartMission) }
    @discardableResult static func abortLanding(_ climbMeters: Double) -> Bool { Qgc.invoke(vehicleAbortLanding, climbMeters) }
    @discardableResult static func gripper(_ action: Int) -> Bool { Qgc.invoke(vehicleGripper, action) }
    @discardableResult static func stopRoi() -> Bool { Qgc.invoke(vehicleStopRoi) }
    @discardableResult static func forceArm() -> Bool { Qgc.invoke(vehicleForceArm) }
    @discardableResult static func emergencyStop() -> Bool { Qgc.invoke(vehicleEmergencyStop) }
    @discardableResult static func motorTest(_ motor: Int, percent: Int, seconds: Int, inOrder: Bool) -> Bool { Qgc.invoke(vehicleMotorTest, motor, percent, seconds, inOrder) }
    @discardableResult static func overrideRcChannel(_ channel: Int, pwm: Int) -> Bool { Qgc.invoke(vehicleRcOverride, channel, pwm) }
    @discardableResult static func releaseRcChannel(_ channel: Int) -> Bool { Qgc.invoke(vehicleRcRelease, channel) }
    @discardableResult static func clearRcOverrides() -> Bool { Qgc.invoke(vehicleRcClear) }
    @discardableResult static func declareRemoteIdEmergency(_ declare: Bool) -> Bool { Qgc.invoke(vehicleRemoteIdEmergency, declare) }
    @discardableResult static func resetAllMessages() -> Bool { Qgc.invoke(vehicleResetMessages) }
    @discardableResult static func clearMessages() -> Bool { Qgc.invoke(vehicleClearMessages) }
}
