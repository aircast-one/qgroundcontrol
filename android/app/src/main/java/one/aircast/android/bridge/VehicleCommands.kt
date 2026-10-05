package one.aircast.android.bridge

import org.json.JSONObject

private const val VEHICLE_ARMED = "vehicle.armed"
private const val VEHICLE_FLIGHT_MODE = "vehicle.flightMode"
private const val VEHICLE_CHECKLIST_STATE = "vehicle.checkListState"
private const val VEHICLE_VTOL_FORWARD_FLIGHT = "vehicle.vtolInFwdFlight"
private const val VEHICLE_HEADING = "vehicle.heading"
private const val VEHICLE_COORDINATE = "vehicle.coordinate"
private const val VEHICLE_TAKEOFF = "vehicle.guidedModeTakeoff"
private const val VEHICLE_LAND = "vehicle.guidedModeLand"
private const val VEHICLE_RETURN_TO_LAUNCH = "vehicle.guidedModeRTL"
private const val VEHICLE_CHANGE_ALTITUDE = "vehicle.guidedModeChangeAltitude"
private const val VEHICLE_START_MISSION = "vehicle.startMission"
private const val VEHICLE_ABORT_LANDING = "vehicle.abortLanding"
private const val VEHICLE_GRIPPER = "vehicle.sendGripperAction"
private const val VEHICLE_STOP_ROI = "vehicle.stopGuidedModeROI"
private const val VEHICLE_FORCE_ARM = "vehicle.forceArm"
private const val VEHICLE_EMERGENCY_STOP = "vehicle.emergencyStop"
private const val VEHICLE_MOTOR_TEST = "vehicle.motorTest"
private const val VEHICLE_RC_OVERRIDE = "vehicle.setRcChannelOverride"
private const val VEHICLE_RC_RELEASE = "vehicle.releaseRcChannelOverride"
private const val VEHICLE_RC_CLEAR = "vehicle.clearRcChannelOverrides"
private const val VEHICLE_REMOTE_ID_EMERGENCY = "vehicle.remoteIDManager.setEmergency"
private const val VEHICLE_RESET_MESSAGES = "vehicle.resetAllMessages"
private const val VEHICLE_CLEAR_MESSAGES = "vehicle.clearMessages"

internal object VehicleCommands {
    fun setArmed(armed: Boolean): Boolean = Qgc.set(VEHICLE_ARMED, armed)
    fun setFlightMode(mode: String): Boolean = Qgc.set(VEHICLE_FLIGHT_MODE, mode)
    fun setChecklistState(state: Int): Boolean = Qgc.set(VEHICLE_CHECKLIST_STATE, state)
    fun setForwardFlight(forward: Boolean): Boolean = Qgc.set(VEHICLE_VTOL_FORWARD_FLIGHT, forward)

    fun heading(): JSONObject = Qgc.get(VEHICLE_HEADING)
    fun coordinate(): JSONObject = Qgc.get(VEHICLE_COORDINATE)

    fun takeoff(): Boolean = Qgc.invoke(VEHICLE_TAKEOFF)
    fun takeoff(meters: Double): Boolean = Qgc.invoke(VEHICLE_TAKEOFF, meters)
    fun takeoffRefusal(): String? = Qgc.refusalOf(VEHICLE_TAKEOFF)
    fun takeoffRefusal(meters: Double): String? = Qgc.refusalOf(VEHICLE_TAKEOFF, meters)
    fun land(): Boolean = Qgc.invoke(VEHICLE_LAND)
    fun landRefusal(): String? = Qgc.refusalOf(VEHICLE_LAND)
    fun returnToLaunch(smart: Boolean): Boolean = Qgc.invoke(VEHICLE_RETURN_TO_LAUNCH, smart)
    fun changeSpeed(command: String, metersSecond: Double): Boolean =
        // qtpaths: vehicle.guidedModeChangeGroundSpeedMetersSecond, vehicle.guidedModeChangeEquivalentAirspeedMetersSecond
        Qgc.invoke("vehicle.$command", metersSecond)
    fun changeAltitude(deltaMeters: Double, pause: Boolean): Boolean = Qgc.invoke(VEHICLE_CHANGE_ALTITUDE, deltaMeters, pause)
    fun startMission(): Boolean = Qgc.invoke(VEHICLE_START_MISSION)
    fun abortLanding(climbMeters: Double): Boolean = Qgc.invoke(VEHICLE_ABORT_LANDING, climbMeters)
    fun gripper(action: Int): Boolean = Qgc.invoke(VEHICLE_GRIPPER, action)
    fun stopRoi(): Boolean = Qgc.invoke(VEHICLE_STOP_ROI)
    fun forceArm(): Boolean = Qgc.invoke(VEHICLE_FORCE_ARM)
    fun emergencyStop(): Boolean = Qgc.invoke(VEHICLE_EMERGENCY_STOP)
    fun motorTest(motor: Int, percent: Int, seconds: Int, inOrder: Boolean): Boolean = Qgc.invoke(VEHICLE_MOTOR_TEST, motor, percent, seconds, inOrder)
    fun overrideRcChannel(channel: Int, pwm: Int): Boolean = Qgc.invoke(VEHICLE_RC_OVERRIDE, channel, pwm)
    fun releaseRcChannel(channel: Int): Boolean = Qgc.invoke(VEHICLE_RC_RELEASE, channel)
    fun clearRcOverrides(): Boolean = Qgc.invoke(VEHICLE_RC_CLEAR)
    fun declareRemoteIdEmergency(declare: Boolean): Boolean = Qgc.invoke(VEHICLE_REMOTE_ID_EMERGENCY, declare)
    fun resetAllMessages(): Boolean = Qgc.invoke(VEHICLE_RESET_MESSAGES)
    fun clearMessages(): Boolean = Qgc.invoke(VEHICLE_CLEAR_MESSAGES)
}
