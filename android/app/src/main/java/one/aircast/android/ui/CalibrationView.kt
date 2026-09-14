package one.aircast.android.ui

import org.json.JSONObject
import one.aircast.mapspike.optText

internal const val CALIBRATION = "view.calibration"

internal data class CalibrationSide(
    val key: String,
    val title: String,
    val visible: Boolean,
    val stage: String,
    val rotate: Boolean,
)

internal data class CalibrationRoutine(
    val id: String,
    val title: String,
    val invocation: String,
    val arguments: List<Boolean>,
    val blocked: Boolean,
    val enabled: Boolean,
    val description: String,
    val warning: String,
    val spinsPropeller: Boolean,
)

internal data class CalibrationState(
    val connected: Boolean,
    val inProgress: Boolean,
    val showsSides: Boolean,
    val nextEnabled: Boolean,
    val cancelEnabled: Boolean,
    val progress: Double,
    val helpText: String,
    val statusText: String,
    val accelNeeded: Boolean,
    val compassNeeded: Boolean,
    val needsAttention: String,
    val sides: List<CalibrationSide>,
    val routines: List<CalibrationRoutine>,
)

private fun <T> JSONObject.list(key: String, item: (JSONObject) -> T): List<T> =
    optJSONArray(key)?.let { array ->
        (0 until array.length()).mapNotNull { array.optJSONObject(it)?.let(item) }
    } ?: emptyList()

internal fun calibrationState(view: JSONObject?): CalibrationState? {
    if (view == null || view.optText("class") != "Calibration") return null
    return CalibrationState(
        connected = view.optBoolean("connected"),
        inProgress = view.optBoolean("inProgress"),
        showsSides = view.optBoolean("showsSides"),
        nextEnabled = view.optBoolean("nextEnabled"),
        cancelEnabled = view.optBoolean("cancelEnabled"),
        progress = view.optDouble("progress", 0.0),
        helpText = view.optText("helpText"),
        statusText = view.optText("statusText"),
        accelNeeded = view.optBoolean("accelNeeded"),
        compassNeeded = view.optBoolean("compassNeeded"),
        needsAttention = view.optText("needsAttention"),
        sides = view.list("sides") {
            CalibrationSide(
                key = it.optText("key"),
                title = it.optText("title"),
                visible = it.optBoolean("visible"),
                stage = it.optText("stage"),
                rotate = it.optBoolean("rotate"),
            )
        },
        routines = view.list("routines") {
            CalibrationRoutine(
                id = it.optText("id"),
                title = it.optText("title"),
                invocation = it.optText("invocation"),
                arguments = it.optJSONArray("arguments")?.let { args ->
                    (0 until args.length()).map { index -> args.optBoolean(index) }
                } ?: emptyList(),
                blocked = it.optBoolean("blocked"),
                enabled = it.optBoolean("enabled"),
                description = it.optText("description"),
                warning = it.optText("warning"),
                spinsPropeller = it.optBoolean("spinsPropeller"),
            )
        },
    )
}

internal fun routineStatus(routine: CalibrationRoutine, state: CalibrationState): String = when {
    routine.blocked -> "Calibrate the accelerometer first"
    routine.id == "accelerometer" -> if (state.accelNeeded) "Not calibrated" else "Calibrated"
    routine.id == "compass" -> if (state.compassNeeded) "Not calibrated" else "Calibrated"
    else -> ""
}

internal const val COMPASS_ROUTINE = "compass"

internal const val REBOOT_VEHICLE = "vehicle.rebootVehicle"

internal fun runningTitle(name: String): String =
    name.trim().takeIf { it.isNotBlank() }?.let { "Calibrating $it" } ?: "Calibration in progress"

internal const val SENSOR_HEALTH = "view.sensors"

internal data class SensorHealth(val name: String, val state: String, val label: String)

internal data class SensorHealthReading(
    val available: Boolean,
    val sensors: List<SensorHealth>,
    val failing: List<String>,
    val status: String,
)

internal fun sensorHealth(view: JSONObject?): SensorHealthReading? {
    if (view == null || view.optText("class") != "SensorHealth") return null
    val listed = view.optJSONArray("sensors")
    return SensorHealthReading(
        available = view.optBoolean("available"),
        sensors = (0 until (listed?.length() ?: 0)).mapNotNull { index ->
            listed?.optJSONObject(index)?.let { item ->
                item.optText("name").takeIf { it.isNotBlank() }?.let { name ->
                    SensorHealth(name, item.optText("state"), item.optText("label"))
                }
            }
        },
        failing = view.optJSONArray("failing")?.let { array ->
            (0 until array.length()).map { array.optString(it) }.filter { it.isNotBlank() }
        } ?: emptyList(),
        status = view.optText("status"),
    )
}

internal fun healthSummary(reading: SensorHealthReading?): String = when {
    reading == null || !reading.available -> ""
    reading.failing.size == 1 -> "${reading.failing.first()} is reporting a fault."
    reading.failing.size > 1 -> "${reading.failing.size} sensors are reporting faults."
    else -> ""
}
