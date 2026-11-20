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
private const val VEHICLE_PAUSE = "vehicle.pauseVehicle"
private const val VEHICLE_START_MISSION = "vehicle.startMission"
private const val VEHICLE_ABORT_LANDING = "vehicle.abortLanding"
private const val VEHICLE_GRIPPER = "vehicle.sendGripperAction"
private const val VEHICLE_FIX_PAYLOAD = "vehicle.fixPayload"
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

    fun heading(): JSONObject = Qgc.get(VEHICLE_HEADING)
    fun coordinate(): JSONObject = Qgc.get(VEHICLE_COORDINATE)

    fun takeoffRefusal(): String? = Qgc.refusalOf(VEHICLE_TAKEOFF)
    fun takeoffRefusal(meters: Double): String? = Qgc.refusalOf(VEHICLE_TAKEOFF, meters)
    fun landRefusal(): String? = Qgc.refusalOf(VEHICLE_LAND)
    fun returnToLaunch(smart: Boolean): Boolean = Qgc.invoke(VEHICLE_RETURN_TO_LAUNCH, smart)
    fun returnToLaunchRefusal(smart: Boolean): String? = Qgc.refusalOf(VEHICLE_RETURN_TO_LAUNCH, smart)
    fun pauseRefusal(): String? = Qgc.refusalOf(VEHICLE_PAUSE)
    fun changeSpeedRefusal(command: String, metersSecond: Double): String? =
        // qtpaths: vehicle.guidedModeChangeGroundSpeedMetersSecond, vehicle.guidedModeChangeEquivalentAirspeedMetersSecond
        Qgc.refusalOf("vehicle.$command", metersSecond)
    fun changeAltitudeRefusal(deltaMeters: Double, pause: Boolean): String? = Qgc.refusalOf(VEHICLE_CHANGE_ALTITUDE, deltaMeters, pause)
    fun startMissionRefusal(): String? = Qgc.refusalOf(VEHICLE_START_MISSION)
    fun abortLandingRefusal(climbMeters: Double): String? = Qgc.refusalOf(VEHICLE_ABORT_LANDING, climbMeters)
    fun gripperRefusal(action: Int, bay: Int = 0): String? = Qgc.refusalOf(VEHICLE_GRIPPER, action, bay)
    fun fixPayloadRefusal(code: String): String? = Qgc.refusalOf(VEHICLE_FIX_PAYLOAD, code)
    fun stopRoiRefusal(): String? = Qgc.refusalOf(VEHICLE_STOP_ROI)
    fun forceArmRefusal(): String? = Qgc.refusalOf(VEHICLE_FORCE_ARM)
    fun forwardFlightRefusal(forward: Boolean): String? = Qgc.writeRefusal(VEHICLE_VTOL_FORWARD_FLIGHT, forward)
    fun emergencyStop(): Boolean = Qgc.invoke(VEHICLE_EMERGENCY_STOP)
    fun motorTest(motor: Int, percent: Int, seconds: Int, inOrder: Boolean): Boolean = Qgc.invoke(VEHICLE_MOTOR_TEST, motor, percent, seconds, inOrder)
    fun overrideRcChannel(channel: Int, pwm: Int): Boolean = Qgc.invoke(VEHICLE_RC_OVERRIDE, channel, pwm)
    fun releaseRcChannel(channel: Int): Boolean = Qgc.invoke(VEHICLE_RC_RELEASE, channel)
    fun clearRcOverrides(): Boolean = Qgc.invoke(VEHICLE_RC_CLEAR)
    fun declareRemoteIdEmergency(declare: Boolean): Boolean = Qgc.invoke(VEHICLE_REMOTE_ID_EMERGENCY, declare)
    fun resetAllMessages(): Boolean = Qgc.invoke(VEHICLE_RESET_MESSAGES)
    fun clearMessages(): Boolean = Qgc.invoke(VEHICLE_CLEAR_MESSAGES)
}
