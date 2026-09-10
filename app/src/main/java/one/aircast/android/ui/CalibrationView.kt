package one.aircast.android.ui

import org.json.JSONObject

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
    if (view == null || view.optString("class") != "Calibration") return null
    return CalibrationState(
        connected = view.optBoolean("connected"),
        inProgress = view.optBoolean("inProgress"),
        showsSides = view.optBoolean("showsSides"),
        nextEnabled = view.optBoolean("nextEnabled"),
        cancelEnabled = view.optBoolean("cancelEnabled"),
        progress = view.optDouble("progress", 0.0),
        helpText = view.optString("helpText"),
        statusText = view.optString("statusText"),
        accelNeeded = view.optBoolean("accelNeeded"),
        compassNeeded = view.optBoolean("compassNeeded"),
        needsAttention = view.optString("needsAttention"),
        sides = view.list("sides") {
            CalibrationSide(
                key = it.optString("key"),
                title = it.optString("title"),
                visible = it.optBoolean("visible"),
                stage = it.optString("stage"),
                rotate = it.optBoolean("rotate"),
            )
        },
        routines = view.list("routines") {
            CalibrationRoutine(
                id = it.optString("id"),
                title = it.optString("title"),
                invocation = it.optString("invocation"),
                arguments = it.optJSONArray("arguments")?.let { args ->
                    (0 until args.length()).map { index -> args.optBoolean(index) }
                } ?: emptyList(),
                blocked = it.optBoolean("blocked"),
                enabled = it.optBoolean("enabled"),
                description = it.optString("description"),
                warning = it.optString("warning"),
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
