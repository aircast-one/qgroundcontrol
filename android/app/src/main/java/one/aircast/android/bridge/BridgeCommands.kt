package one.aircast.android.bridge

import org.json.JSONObject

private const val LINKS_COMMIT = "links.commitLinkConfigurations"
private const val LINKS_BLUETOOTH_SCAN = "links.bluetoothScan"
private const val LINKS_CONNECT = "links.createConnectedLink"
private const val LINKS_REMOVE = "links.removeConfiguration"
private const val LINKS_CREATE_CLOUD = "links.createAircastCloudLink"
private const val LINKS_CREATE_REPLAY = "links.createLogReplayConfiguration"
private const val LINKS_CREATE_BLUETOOTH = "links.createBluetoothLink"
private const val LINKS_CREATE_SERIAL = "links.createSerialConfiguration"
private const val LINKS_CREATE_SUPPORT = "links.createMavlinkForwardingSupportLink"
private const val LINKS_START_MOCK = "links.startMockLink"
private const val ACCOUNT_STATE = "account"
private const val ACCOUNT_API_BASE = "account.apiBase"
private const val ACCOUNT_SIGN_IN = "account.signIn"
private const val ACCOUNT_SIGN_OUT = "account.signOut"
private const val ACCOUNT_CANCEL_SIGN_IN = "account.cancelSignIn"
private const val PLAN_REDO = "plan.redo"
private const val PLAN_UNDO_TRACKING = "plan.undoTracking"
private const val PLAN_REMOVE_ALL_FROM_VEHICLE = "plan.removeAllFromVehicle"
private const val PLAN_RESUME_MISSION = "planFly.missionController.resumeMission"
private const val PLAN_COMMAND_CATEGORIES = "missionCommandTree.categoriesForVehicle"
private const val PLAN_COMMANDS_FOR_CATEGORY = "missionCommandTree.getCommandsForCategory"
private const val MISSION_COMPLETE_DISMISS = "missionComplete.dismiss"
private const val VIDEO_NATIVE_RENDERING = "video.setNativeRendering"
private const val VIDEO_INIT_NATIVE = "video.initNative"
private const val VIDEO_ACTIVE_SOURCE = "video.setActiveVideoSource"
private const val VIDEO_START_RECORDING = "video.startRecording"
private const val VIDEO_STOP_RECORDING = "video.stopRecording"
private const val VIDEO_RESTART = "video.restart"
private const val VIDEO_DEVICE_CAMERA_ROTATION = "video.setDeviceCameraRotation"
private const val VIDEO_PICTURE_IN_PICTURE = "settings.videoSettings.multiViewEnabled"
private const val VIDEO_PIP_SHOWN = "video.setPipShown"
private const val VIDEO_STREAM_ENABLED = "settings.videoSettings.streamEnabled"
private const val CAMERAS_ADD = "cameras.add"
private const val CAMERAS_UPDATE = "cameras.update"
private const val CAMERAS_REMOVE = "cameras.remove"
private const val CAMERAS_MOVE = "cameras.move"
private const val VEHICLE_FIELDS = "vehicle"
private const val ESC_CALIBRATION_START = "escCalibration.start"
private const val ESC_CALIBRATION_CLOSE = "escCalibration.close"
private const val SUB_MOTORS_OPEN = "apmSubMotors.open"
private const val SUB_MOTORS_TEST = "apmSubMotors.test"
private const val PACKET_RADIO_REFRESH = "packetRadio.refreshAdapters"
private const val INSPECTOR_MESSAGE_INTERVAL = "mavlinkInspector.setMessageInterval"
private const val INSPECTOR_ACTIVE_SYSTEM = "mavlinkInspector.setActiveSystem"
private const val GIMBAL_CONTROL = "gimbal.control"
private const val FIRST_RUN_SHOWN = "firstRun.markShown"
private const val SUBTITLES_INSTRUMENTS = "subtitles.setInstruments"
private const val HOST_ACKNOWLEDGE = "host.acknowledgeThrough"

internal object LinkCommands {
    fun commitConfigurations(): Boolean = Qgc.invoke(LINKS_COMMIT)
    fun scanBluetooth(scanning: Boolean): Boolean = Qgc.invoke(LINKS_BLUETOOTH_SCAN, scanning)
    fun connect(configuration: String): Boolean = Qgc.invoke(LINKS_CONNECT, configuration)
    fun remove(configuration: String): Boolean = Qgc.invoke(LINKS_REMOVE, configuration)
    fun createAircastCloud(name: String, apiBase: String, deviceId: String): Boolean = Qgc.invokeResult(LINKS_CREATE_CLOUD, name, apiBase, deviceId) == true
    fun createLogReplay(name: String, log: String): Boolean = Qgc.invokeResult(LINKS_CREATE_REPLAY, name, log) == true
    fun createBluetooth(name: String, deviceName: String, address: String): Boolean = Qgc.invokeResult(LINKS_CREATE_BLUETOOTH, name, deviceName, address) == true
    fun createSerial(name: String, port: String, baud: Int): Boolean = Qgc.invokeResult(LINKS_CREATE_SERIAL, name, port, baud) == true
    fun createSupportForwarding(): Boolean = Qgc.invoke(LINKS_CREATE_SUPPORT)
    fun startMock(arguments: List<Any>): Boolean = Qgc.invokeResult(LINKS_START_MOCK, *arguments.toTypedArray()) == true
}

internal object AccountCommands {
    fun state(): JSONObject = Qgc.get(ACCOUNT_STATE)
    fun setApiBase(apiBase: String): Boolean = Qgc.set(ACCOUNT_API_BASE, apiBase)
    fun signIn(): Boolean = Qgc.invoke(ACCOUNT_SIGN_IN)
    fun signOut(): Boolean = Qgc.invoke(ACCOUNT_SIGN_OUT)
    fun cancelSignIn(): Boolean = Qgc.invoke(ACCOUNT_CANCEL_SIGN_IN)
}

internal object PlanCommands {
    fun redo(): Boolean = Qgc.invoke(PLAN_REDO)
    fun setUndoTracking(tracking: Boolean): Boolean = Qgc.set(PLAN_UNDO_TRACKING, tracking)
    fun removeAllFromVehicle(): Boolean = Qgc.invoke(PLAN_REMOVE_ALL_FROM_VEHICLE)
    fun resumeMission(waypoint: Int): Boolean = Qgc.invoke(PLAN_RESUME_MISSION, waypoint)
    fun dismissMissionComplete(notice: Long): Boolean = Qgc.invoke(MISSION_COMPLETE_DISMISS, notice)
    fun commandCategories(): Any? = Qgc.invokeResult(PLAN_COMMAND_CATEGORIES)
    fun commandsForCategory(category: String): Any? = Qgc.invokeResult(PLAN_COMMANDS_FOR_CATEGORY, null, category, true)
    fun setLandingHeading(index: Int, heading: Double): String? = Qgc.writeRefusal("plan.missionController.visualItems.$index.landingHeading", heading)
    fun setLandingCoordinate(index: Int, coordinate: JSONObject): String? = Qgc.writeRefusal("plan.missionController.visualItems.$index.landingCoordinate", coordinate)
    fun setAltitudesRelative(index: Int, relative: Boolean): String? = Qgc.writeRefusal("plan.missionController.visualItems.$index.altitudesAreRelative", relative)
}

internal object VideoCommands {
    fun setNativeRendering(native: Boolean): Boolean = Qgc.invoke(VIDEO_NATIVE_RENDERING, native)
    fun initNative(): Boolean = Qgc.invoke(VIDEO_INIT_NATIVE)
    fun setActiveSource(slot: Int): Boolean = Qgc.invoke(VIDEO_ACTIVE_SOURCE, slot)
    fun setRecording(recording: Boolean): Boolean = Qgc.invoke(if (recording) VIDEO_START_RECORDING else VIDEO_STOP_RECORDING)
    fun restart(): Boolean = Qgc.invoke(VIDEO_RESTART)
    fun setDeviceCameraRotation(degrees: Int): Boolean = Qgc.invoke(VIDEO_DEVICE_CAMERA_ROTATION, degrees)
    fun setPictureInPicture(shown: Boolean): Boolean = Qgc.set(VIDEO_PICTURE_IN_PICTURE, shown)
    fun setPipShown(shown: Boolean): Boolean = Qgc.invoke(VIDEO_PIP_SHOWN, shown)
    fun turnStreamOn(): Boolean = Qgc.set(VIDEO_STREAM_ENABLED, true)
}

internal object CameraCommands {
    fun add(name: String, source: String, url: String): String? = Qgc.refusalOf(CAMERAS_ADD, name, source, url)
    fun update(slot: Int, name: String, source: String, url: String): String? = Qgc.refusalOf(CAMERAS_UPDATE, slot, name, source, url)
    fun remove(slot: Int): String? = Qgc.refusalOf(CAMERAS_REMOVE, slot)
    fun move(from: Int, to: Int): String? = Qgc.refusalOf(CAMERAS_MOVE, from, to)
}

internal object SetupCommands {
    fun vehicleFields(fields: Collection<String>): JSONObject = Qgc.get(VEHICLE_FIELDS, fields)
    fun startEscCalibration(): Boolean = Qgc.invoke(ESC_CALIBRATION_START)
    fun closeEscCalibration(): Boolean = Qgc.invoke(ESC_CALIBRATION_CLOSE)
    fun openSubMotors(): Boolean = Qgc.invoke(SUB_MOTORS_OPEN)
    fun testSubMotor(motor: Int, value: Double): Boolean = Qgc.invoke(SUB_MOTORS_TEST, motor, value)
    fun refreshPacketRadios(): Boolean = Qgc.invoke(PACKET_RADIO_REFRESH)
    fun setInspectorMessageInterval(rate: Int): Boolean = Qgc.invoke(INSPECTOR_MESSAGE_INTERVAL, rate)
    fun setInspectorSystem(system: Int): Boolean = Qgc.invoke(INSPECTOR_ACTIVE_SYSTEM, system)
    fun takeGimbalControlRefusal(): String? = Qgc.refusalOf(GIMBAL_CONTROL, true)
}

internal object AppCommands {
    fun markFirstRunShown(): Boolean = Qgc.invoke(FIRST_RUN_SHOWN)
    fun setSubtitleInstruments(vehicleClass: String, instruments: List<String>): Boolean = Qgc.invoke(SUBTITLES_INSTRUMENTS, vehicleClass, instruments.joinToString(","))
    fun acknowledgeNoticesThrough(through: Long): Boolean = Qgc.invoke(HOST_ACKNOWLEDGE, through)
}
