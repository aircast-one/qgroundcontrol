package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.runtime.rememberCoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath

private const val CAL = "sensorsCal"

private const val CAL_START_MS = 5000L

internal fun calibrationFailure(name: String, started: Boolean): String? =
    if (started) null else "$name calibration did not start."

private fun calibrationStatus(): String =
    calibrationState(Qgc.get(CALIBRATION))?.statusText.orEmpty()

private fun calibrationRunning(): Boolean =
    calibrationState(Qgc.get(CALIBRATION))?.inProgress == true

internal fun calibrationBegan(running: Boolean, statusBefore: String, statusNow: String): Boolean =
    running || statusNow != statusBefore

internal data class RoutineCopy(val instruction: String, val warning: String = "")

internal val ROUTINE_COPY = mapOf(
    "accelerometer" to RoutineCopy(
        "You will be asked to hold the vehicle still in six orientations. " +
            "Press Next once it is steady in each one.",
    ),
    "compass" to RoutineCopy(
        "Rotate the vehicle slowly around all axes until the bar fills. " +
            "Stand away from metal, cars and reinforced concrete.",
    ),
    "levelHorizon" to RoutineCopy(
        "Place the vehicle in its level flight position and leave it still.",
        "Sets what the vehicle considers level. Get this wrong and it will drift in flight.",
    ),
    "gyro" to RoutineCopy(
        "Place the vehicle on a surface and leave it completely still.",
    ),
    "pressure" to RoutineCopy(
        "Zeroes the altitude at the current pressure. Do this where you will take off.",
    ),
)

internal fun routineCopy(routine: CalibrationRoutine): RoutineCopy =
    ROUTINE_COPY[routine.id] ?: RoutineCopy(routine.description, routine.warning)

@Composable
private fun SensorsNotice(text: String, modifier: Modifier = Modifier) {
    Text(
        text = text,
        style = MaterialTheme.typography.bodyLarge,
        textAlign = TextAlign.Center,
        modifier = modifier
            .fillMaxWidth()
            .padding(24.dp),
    )
}

@Composable
private fun StartDialog(
    calibration: CalibrationRoutine,
    onConfirm: () -> Unit,
    onDismiss: () -> Unit,
) {
    val copy = routineCopy(calibration)
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Calibrate ${calibration.title}?") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Text(copy.instruction)
                if (copy.warning.isNotBlank()) {
                    Text(
                        text = copy.warning,
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.error,
                    )
                }
            }
        },
        confirmButton = {
            TextButton(onClick = { onConfirm(); onDismiss() }) { Text("Start") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

@Composable
private fun OrientationTile(label: String, done: Boolean, inProgress: Boolean, rotate: Boolean) {
    val status = when {
        inProgress && rotate -> "Rotate"
        inProgress -> "Hold still"
        done -> "Done"
        else -> "Pending"
    }
    val tint = when {
        inProgress -> MaterialTheme.colorScheme.primary
        done -> MaterialTheme.colorScheme.secondary
        else -> MaterialTheme.colorScheme.onSurfaceVariant
    }
    Card(
        colors = CardDefaults.cardColors(
            containerColor = if (inProgress) {
                MaterialTheme.colorScheme.primaryContainer
            } else {
                MaterialTheme.colorScheme.surfaceVariant
            },
        ),
        modifier = Modifier
            .fillMaxWidth()
            .aspectRatio(1.9f),
    ) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(8.dp),
            verticalArrangement = Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(label, style = MaterialTheme.typography.labelLarge)
            Text(status, style = MaterialTheme.typography.bodySmall, color = tint)
        }
    }
}

@Composable
private fun OrientationGrid(sides: List<CalibrationSide>) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        sides.filter { it.visible }.chunked(2).forEach { row ->
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                row.forEach { side ->
                    Box(Modifier.weight(1f)) {
                        OrientationTile(
                            label = side.title,
                            done = side.stage == "done",
                            inProgress = side.stage == "inProgress",
                            rotate = side.rotate,
                        )
                    }
                }
                repeat(2 - row.size) { Box(Modifier.weight(1f)) {} }
            }
        }
    }
}

@Composable
private fun RunningCalibration(
    name: String,
    state: CalibrationState,
    modifier: Modifier = Modifier,
) {
    val progress = state.progress
    val helpText = state.helpText
    val statusText = state.statusText

    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Text(runningTitle(name), style = MaterialTheme.typography.titleMedium)

        LinearProgressIndicator(
            progress = { progress.toFloat().coerceIn(0f, 1f) },
            modifier = Modifier.fillMaxWidth(),
        )

        if (helpText.isNotBlank()) {
            Text(helpText, style = MaterialTheme.typography.bodyMedium)
        }

        if (state.showsSides) {
            OrientationGrid(state.sides)
        }

        if (statusText.isNotBlank()) {
            Text(
                text = statusText,
                style = MaterialTheme.typography.bodySmall,
                fontFamily = FontFamily.Monospace,
            )
        }

        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Button(
                onClick = { offMainDetached { Qgc.invoke("$CAL.nextClicked") } },
                enabled = state.nextEnabled,
                modifier = Modifier.weight(1f),
            ) { Text("Next") }
            OutlinedButton(
                onClick = { offMainDetached { Qgc.invoke("$CAL.cancelCalibration") } },
                enabled = state.cancelEnabled,
                modifier = Modifier.weight(1f),
            ) { Text("Cancel") }
        }
    }
}

@Composable
fun SensorsScreen(modifier: Modifier = Modifier) {
    val hasVehicle = hasVehicle()
    val setupJson by qgcPath(SETUP)
    val isPx4 = remember(setupJson) { isPx4(setupReadiness(setupJson)) }
    val json by qgcPath(CALIBRATION)
    val healthJson by qgcPath(SENSOR_HEALTH)
    val health = remember(healthJson) { sensorHealth(healthJson) }
    val state = remember(json) { calibrationState(json) }
    var pending by remember { mutableStateOf<CalibrationRoutine?>(null) }
    var runningName by remember { mutableStateOf("") }
    var rebootOffered by remember { mutableStateOf(false) }
    var notice by remember { mutableStateOf<String?>(null) }
    val flyJson by qgcPath(FLY_STATE)
    val aloft = remember(flyJson) { flyState(flyJson)?.state == "flying" }
    val scope = rememberCoroutineScope()

    if (!hasVehicle) {
        SensorsNotice("Connect a vehicle to calibrate its sensors.", modifier)
        return
    }

    if (isPx4) {
        SensorsNotice(
            "Sensor calibration is only carried over for ArduPilot vehicles. " +
                "Use QGroundControl on a computer for this vehicle.",
            modifier,
        )
        return
    }

    if (state == null) {
        SensorsNotice("Reading the vehicle's calibration state.", modifier)
        return
    }

    pending?.let { calibration ->
        StartDialog(
            calibration = calibration,
            onConfirm = {
                runningName = calibration.title
                rebootOffered = rebootOffered || calibration.id == COMPASS_ROUTINE
                notice = null
                scope.launch {
                    val before = withContext(Dispatchers.Default) { calibrationStatus() }
                    val dispatched = withContext(Dispatchers.Default) {
                        Qgc.invoke(calibration.invocation, *calibration.arguments.toTypedArray())
                    }
                    val started = dispatched && withTimeoutOrNull(CAL_START_MS) {
                        while (
                            !withContext(Dispatchers.Default) {
                                calibrationBegan(calibrationRunning(), before, calibrationStatus())
                            }
                        ) {
                            delay(150)
                        }
                        true
                    } == true
                    notice = calibrationFailure(calibration.title, started)
                }
            },
            onDismiss = { pending = null },
        )
    }

    val running = state.inProgress
    DisposableEffect(running) {
        onDispose {
            if (running) {
                offMainDetached { Qgc.invoke("$CAL.cancelCalibration") }
            }
        }
    }

    if (running) {
        RunningCalibration(runningName, state, modifier)
        return
    }

    LazyColumn(
        modifier = modifier.fillMaxSize(),
    ) {
        health?.takeIf { it.available && it.sensors.isNotEmpty() }?.let { reading ->
            item(key = "health") {
                Column {
                    SectionHeader("Sensor health")
                    healthSummary(reading).takeIf { it.isNotBlank() }?.let { line ->
                        Text(
                            text = line,
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.error,
                            modifier = Modifier.padding(horizontal = 20.dp, vertical = 4.dp),
                        )
                    }
                    reading.sensors.forEach { sensor ->
                        SetupRow(
                            title = sensor.name,
                            status = sensor.label,
                            state = when (sensor.state) {
                                "healthy" -> SetupState.Done
                                "unhealthy" -> SetupState.NeedsAttention
                                else -> SetupState.Neutral
                            },
                        )
                    }
                }
            }
        }

        item(key = "header") { SectionHeader("Calibration") }

        notice?.let { message ->
            item(key = "notice") {
                Text(
                    text = message,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.padding(horizontal = 20.dp, vertical = 8.dp),
                )
            }
        }

        items(state.routines, key = { it.id }) { routine ->
            val status = routineStatus(routine, state)
            SetupRow(
                title = routine.title,
                status = if (routine.spinsPropeller) "Spins the motors" else status,
                state = when (status) {
                    "Not calibrated" -> SetupState.NeedsAttention
                    "Calibrated" -> SetupState.Done
                    else -> SetupState.Neutral
                },
                onClick = if (routine.enabled) ({ pending = routine }) else null,
            )
        }

        if (state.statusText.isNotBlank()) {
            item(key = "last") {
                Column {
                    SectionHeader("Last calibration")
                    Text(
                        text = state.statusText,
                        style = MaterialTheme.typography.bodyMedium,
                        fontFamily = FontFamily.Monospace,
                        modifier = Modifier.padding(horizontal = 20.dp),
                    )
                    if (rebootOffered) {
                        Button(
                            onClick = {
                                rebootOffered = false
                                scope.launch {
                                    withContext(Dispatchers.Default) { Qgc.invoke(REBOOT_VEHICLE) }
                                }
                            },
                            modifier = Modifier.padding(horizontal = 20.dp, vertical = 12.dp),
                        ) { Text("Reboot vehicle") }
                    }
                }
            }
        }

        item(key = "footnote") {
            FootNote(
                "Calibrate where the aircraft will fly, away from metal, with the " +
                    "propellers off. CompassMot is the exception and says so when you " +
                    "open it: it runs the motors, with the propellers inverted.",
            )
        }
    }
}
