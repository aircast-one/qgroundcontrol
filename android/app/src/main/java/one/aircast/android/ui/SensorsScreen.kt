package one.aircast.android.ui

import one.aircast.android.R

import androidx.compose.ui.res.painterResource

import androidx.compose.material3.Icon

import androidx.compose.foundation.layout.size

import androidx.compose.material3.CircularProgressIndicator

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
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
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
internal const val SENSOR_FACTORY_RESET = "sensorSettings.factoryReset"

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

internal fun routineCopy(routine: CalibrationRoutine): RoutineCopy =
    RoutineCopy(routine.dialogHelp.ifBlank { routine.description }, routine.warning)

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

internal const val ACCEL_ROUTINE = "accelerometer"
internal const val CALIBRATION_COMPLETE = "Calibration complete"
private const val MUST_REBOOT = "YOU MUST REBOOT YOUR VEHICLE AFTER EACH CALIBRATION."
private const val COMPASS_QUALITY = "Shown in the indicator bars is the quality of the calibration for each compass.\n\n" +
    "- Green indicates a well functioning compass.\n" +
    "- Yellow indicates a questionable compass or calibration.\n" +
    "- Red indicates a compass which should not be used.\n\n"

internal const val PX4_COMPASS_COMPLETE = "Compass Calibration Complete"
internal const val PX4_REBOOT = "Reboot the vehicle prior to flight."

internal fun postCalibrationTitle(routine: String?, px4: Boolean): String =
    if (px4 && routine == COMPASS_ROUTINE) PX4_COMPASS_COMPLETE else CALIBRATION_COMPLETE

internal fun postCalibrationPrompt(routine: String?, helpText: String, px4: Boolean): String? = when {
    helpText != CALIBRATION_COMPLETE -> null
    px4 && routine == COMPASS_ROUTINE -> PX4_REBOOT
    px4 -> null
    routine == COMPASS_ROUTINE -> COMPASS_QUALITY + MUST_REBOOT
    routine == ACCEL_ROUTINE -> MUST_REBOOT
    else -> null
}

@Composable
private fun StartDialog(
    calibration: CalibrationRoutine,
    fast: FastCompass?,
    onConfirm: (String, List<Any>) -> Unit,
    onDismiss: () -> Unit,
) {
    val copy = routineCopy(calibration)
    var simple by remember { mutableStateOf(false) }
    val offersFast = fast?.takeIf { calibration.id == COMPASS_ROUTINE }
    var fastChoice by remember(offersFast != null) { mutableStateOf(offersFast?.let(::initialFastCompassChoice)) }
    val offersSimple = calibration.id == ACCEL_ROUTINE && calibration.arguments.isNotEmpty()
    val orientationFirst = calibration.id == ACCEL_ROUTINE || calibration.id == COMPASS_ROUTINE
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Calibrate ${calibration.title}?") },
        text = {
            Column(Modifier.verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                if (orientationFirst) {
                    SensorSettingsBlock(
                        calibrating = true,
                        showCompasses = calibration.id == COMPASS_ROUTINE,
                        onSimpleAccel = if (offersSimple) ({ simple = it }) else null,
                    )
                }
                Text(copy.instruction)
                offersFast?.let { offered -> fastChoice?.let { choice -> FastCompassBlock(offered, choice) { fastChoice = it } } }
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
            TextButton(onClick = {
                val north = offersFast?.let { offered -> fastChoice?.takeIf { it.enabled }?.let { offered to it } }
                when {
                    north != null -> onConfirm(north.first.invocation, fastCompassArguments(north.first, north.second))
                    offersSimple -> onConfirm(calibration.invocation, listOf(simple))
                    else -> onConfirm(calibration.invocation, calibration.arguments)
                }
                onDismiss()
            }) { Text("Start") }
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

private val CALIBRATION_RING = 180.dp

internal fun routineIcon(id: String): Int = when (id) {
    "accelerometer" -> R.drawable.ic_vibration
    "compass", "compassMot" -> R.drawable.ic_explore
    "levelHorizon" -> R.drawable.ic_straighten
    "gyro" -> R.drawable.ic_sensors
    "pressure" -> R.drawable.ic_height
    "airspeed" -> R.drawable.ic_speed
    else -> R.drawable.ic_build
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
        Text(runningTitle(name), style = MaterialTheme.typography.headlineSmall)

        if (helpText.isNotBlank()) {
            Text(helpText, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }

        Box(Modifier.fillMaxWidth().padding(vertical = 8.dp), contentAlignment = Alignment.Center) {
            CircularProgressIndicator(
                progress = { progress.toFloat().coerceIn(0f, 1f) },
                modifier = Modifier.size(CALIBRATION_RING),
                strokeWidth = 10.dp,
                trackColor = MaterialTheme.colorScheme.surfaceContainerHighest,
            )
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                Icon(painterResource(R.drawable.ic_sensors), null, tint = MaterialTheme.colorScheme.primary, modifier = Modifier.size(40.dp))
                Text("${(progress * 100).toInt()}%", style = MaterialTheme.typography.headlineMedium)
            }
        }

        if (state.showsSides) {
            OrientationGrid(state.sides)
        }

        if (statusText.isNotBlank()) {
            Text(
                text = statusText,
                style = MaterialTheme.typography.bodySmall,
                fontFamily = FontFamily.Monospace,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
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
        if (state.waitingForCancel) {
            Text(CANCEL_WAIT_TITLE, style = MaterialTheme.typography.titleSmall)
            Text(CANCEL_WAIT_TEXT, style = MaterialTheme.typography.bodyMedium)
        }
    }
}

@Composable
fun SensorsScreen(modifier: Modifier = Modifier) {
    val hasVehicle = hasVehicle()
    val json by qgcPath(CALIBRATION)
    val healthJson by qgcPath(SENSOR_HEALTH)
    val health = remember(healthJson) { sensorHealth(healthJson) }
    val state = remember(json) { calibrationState(json) }
    var pending by remember { mutableStateOf<CalibrationRoutine?>(null) }
    var showSettings by remember { mutableStateOf(false) }
    var confirmFactoryReset by remember { mutableStateOf(false) }
    var runningName by remember { mutableStateOf("") }
    var ranRoutine by remember { mutableStateOf<String?>(null) }
    var rebootPrompt by remember { mutableStateOf<String?>(null) }
    var wasInProgress by remember { mutableStateOf(false) }
    var notice by remember { mutableStateOf<String?>(null) }
    val flyJson by qgcPath(FLY_STATE)
    val aloft = remember(flyJson) { flyState(flyJson)?.state == "flying" }
    val scope = rememberCoroutineScope()

    if (!hasVehicle) {
        SensorsNotice("Connect a vehicle to calibrate its sensors.", modifier)
        return
    }

    if (state == null) {
        SensorsNotice("Reading the vehicle's calibration state.", modifier)
        return
    }

    LaunchedEffect(state.inProgress) {
        if (wasInProgress && !state.inProgress) {
            rebootPrompt = postCalibrationPrompt(ranRoutine, state.helpText, state.px4)
        }
        wasInProgress = state.inProgress
    }

    rebootPrompt?.let { prompt ->
        AlertDialog(
            onDismissRequest = { rebootPrompt = null },
            title = { Text(postCalibrationTitle(ranRoutine, state.px4)) },
            text = { Text(prompt) },
            confirmButton = {
                TextButton(onClick = {
                    rebootPrompt = null
                    scope.launch { withContext(Dispatchers.Default) { Qgc.invoke(REBOOT_VEHICLE) } }
                }) { Text("Reboot Vehicle") }
            },
            dismissButton = { TextButton(onClick = { rebootPrompt = null }) { Text("Close") } },
        )
    }

    if (showSettings) {
        AlertDialog(
            onDismissRequest = { showSettings = false },
            title = { Text(state.settingsTitle) },
            text = { Column(Modifier.verticalScroll(rememberScrollState())) { SensorSettingsBlock(calibrating = false, showCompasses = true) } },
            confirmButton = { TextButton(onClick = { showSettings = false }) { Text("Close") } },
        )
    }

    if (confirmFactoryReset) {
        AlertDialog(
            onDismissRequest = { confirmFactoryReset = false },
            title = { Text("Factory reset") },
            text = { Text("Reset every parameter on the vehicle to its factory default?") },
            confirmButton = {
                TextButton(onClick = {
                    confirmFactoryReset = false
                    scope.launch { notice = withContext(Dispatchers.Default) { Qgc.refusalOf(SENSOR_FACTORY_RESET) } }
                }) { Text("Reset") }
            },
            dismissButton = { TextButton(onClick = { confirmFactoryReset = false }) { Text("Cancel") } },
        )
    }

    pending?.let { calibration ->
        StartDialog(
            calibration = calibration,
            fast = state.fastCompass,
            onConfirm = { invocation, arguments ->
                runningName = calibration.title
                ranRoutine = calibration.id
                notice = null
                scope.launch {
                    val before = withContext(Dispatchers.Default) { calibrationStatus() }
                    val dispatched = withContext(Dispatchers.Default) {
                        Qgc.invoke(invocation, *arguments.toTypedArray())
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
    BlocksNavigation(running && state.px4, CALIBRATION_BLOCK)
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
                            icon = R.drawable.ic_sensors,
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
            val status = routine.status
            SetupRow(
                title = routine.title,
                status = if (routine.spinsPropeller) "Spins the motors" else status,
                state = when (status) {
                    "Not calibrated" -> SetupState.NeedsAttention
                    "Calibrated" -> SetupState.Done
                    else -> SetupState.Neutral
                },
                onClick = if (routine.enabled) ({ pending = routine }) else null,
                icon = routineIcon(routine.id),
            )
        }

        item(key = "sensorSettings") {
            SetupRow(title = state.settingsTitle, status = "", state = SetupState.Neutral, onClick = { showSettings = true }, icon = R.drawable.ic_tune)
        }

        if (state.px4) {
            item(key = "factoryReset") {
                SetupRow(title = "Factory reset", status = "", state = SetupState.NeedsAttention, onClick = { confirmFactoryReset = true }, icon = R.drawable.ic_delete)
            }
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
