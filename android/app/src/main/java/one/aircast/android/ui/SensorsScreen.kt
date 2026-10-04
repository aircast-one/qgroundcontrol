package one.aircast.android.ui

import one.aircast.mapspike.aircast
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.ui.graphics.Color
import androidx.compose.material3.FilterChipDefaults
import androidx.compose.material3.FilterChip
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
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
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
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

internal const val PX4_COMPASS_COMPLETE = "Compass calibration complete"
internal const val PX4_REBOOT = "Reboot the vehicle prior to flight."

internal fun postCalibrationTitle(routine: String?, px4: Boolean): String =
    if (px4 && routine == COMPASS_ROUTINE) PX4_COMPASS_COMPLETE else CALIBRATION_COMPLETE

internal fun postCalibrationPrompt(routine: String?, helpText: String, completed: String, px4: Boolean): String? = when {
    px4 -> PX4_REBOOT.takeIf { helpText == CALIBRATION_COMPLETE && routine == COMPASS_ROUTINE }
    completed == COMPASS_ROUTINE -> COMPASS_QUALITY + MUST_REBOOT
    completed == ACCEL_ROUTINE -> MUST_REBOOT
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
        title = { Text(calibration.dialogTitle.ifBlank { calibration.title }) },
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
            }) { Text("OK") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

internal fun positionsText(sides: List<CalibrationSide>): String? {
    val shown = sides.filter { it.visible }
    val reached = shown.count { it.stage == "done" || it.stage == "inProgress" }
    return "${reached.coerceAtLeast(1)} of ${shown.size} positions".takeIf { shown.isNotEmpty() }
}

internal fun sideImage(key: String, rotating: Boolean): Int = when (key) {
    "UpsideDown" -> if (rotating) R.drawable.cal_vehicle_upside_down_rotate else R.drawable.cal_vehicle_upside_down
    "Left" -> if (rotating) R.drawable.cal_vehicle_left_rotate else R.drawable.cal_vehicle_left
    "Right" -> if (rotating) R.drawable.cal_vehicle_right_rotate else R.drawable.cal_vehicle_right
    "NoseDown" -> if (rotating) R.drawable.cal_vehicle_nose_down_rotate else R.drawable.cal_vehicle_nose_down
    "TailDown" -> if (rotating) R.drawable.cal_vehicle_tail_down_rotate else R.drawable.cal_vehicle_tail_down
    else -> if (rotating) R.drawable.cal_vehicle_down_rotate else R.drawable.cal_vehicle_down
}

internal fun sideStateText(side: CalibrationSide): String = when (side.stage) {
    "inProgress" -> if (side.rotate) "Rotate" else "Hold still"
    "done" -> "Done"
    else -> "Pending"
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun OrientationGrid(sides: List<CalibrationSide>, px4: Boolean) {
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        sides.filter { it.visible }.forEach { side ->
            val current = side.stage == "inProgress"
            val done = side.stage == "done"
            val tint = when {
                current -> MaterialTheme.colorScheme.primary
                done -> MaterialTheme.aircast.success
                else -> MaterialTheme.colorScheme.onSurface.copy(alpha = 0.5f)
            }
            Column(
                Modifier
                    .width(104.dp)
                    .border(2.dp, if (current || done) tint else MaterialTheme.colorScheme.outlineVariant, MaterialTheme.shapes.medium)
                    .padding(6.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                androidx.compose.foundation.Image(
                    painterResource(sideImage(side.key, px4 && current && side.rotate)),
                    contentDescription = side.title,
                    modifier = Modifier.height(72.dp),
                )
                Text(side.title, style = MaterialTheme.typography.labelSmall)
                Text(sideStateText(side), style = MaterialTheme.typography.labelMedium, color = tint, fontWeight = if (current) androidx.compose.ui.text.font.FontWeight.Bold else null)
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
        if (state.showsSides) {
            positionsText(state.sides)?.let { Text(it, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.primary) }
        }
        Text(runningTitle(name), style = MaterialTheme.typography.titleLarge)

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
            OrientationGrid(state.sides, state.px4)
        }

        if (statusText.isNotBlank()) {
            Text(
                text = statusText,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }

        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.End)) {
            TextButton(
                onClick = { offMainDetached { Qgc.invoke("$CAL.cancelCalibration") } },
                enabled = state.cancelEnabled,
            ) { Text("Cancel") }
            Button(
                onClick = { offMainDetached { Qgc.invoke("$CAL.nextClicked") } },
                enabled = state.nextEnabled,
            ) { Text("Next") }
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
            rebootPrompt = postCalibrationPrompt(ranRoutine, state.helpText, state.completed, state.px4)
        }
        wasInProgress = state.inProgress
    }

    rebootPrompt?.let { prompt ->
        AlertDialog(
            onDismissRequest = { rebootPrompt = null },
            title = { Text(postCalibrationTitle(ranRoutine, state.px4)) },
            text = {
                Column(Modifier.verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(prompt)
                    if (!state.px4 && state.completed == COMPASS_ROUTINE) state.compassResults.forEach { CompassFitnessBar(it) }
                    if (state.px4 && ranRoutine == COMPASS_ROUTINE) CompassOrientations()
                }
            },
            confirmButton = {
                TextButton(onClick = {
                    rebootPrompt = null
                    scope.launch { withContext(Dispatchers.Default) { Qgc.invoke(REBOOT_VEHICLE) } }
                }) { Text("Reboot vehicle") }
            },
            dismissButton = { TextButton(onClick = { rebootPrompt = null }) { Text("Close") } },
        )
    }

    if (showSettings) {
        AlertDialog(
            onDismissRequest = { showSettings = false },
            title = { Text(state.settingsDialogTitle.ifBlank { state.settingsTitle }) },
            text = { Column(Modifier.verticalScroll(rememberScrollState())) { SensorSettingsBlock(calibrating = false, showCompasses = true) } },
            confirmButton = { TextButton(onClick = { showSettings = false }) { Text("OK") } },
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
    val stillRunning by rememberUpdatedState(running)
    DisposableEffect(Unit) {
        onDispose {
            if (stillRunning) {
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
                title = sentenceCase(routine.title),
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
            SetupRow(title = sentenceCase(state.settingsTitle), status = "", state = SetupState.Neutral, onClick = { showSettings = true }, icon = R.drawable.ic_tune)
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

@Composable
private fun CompassFitnessBar(result: CompassResult) {
    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Text("Compass ${result.compass}", style = MaterialTheme.typography.labelMedium)
        androidx.compose.foundation.layout.BoxWithConstraints(Modifier.fillMaxWidth().height(16.dp)) {
            Row(Modifier.fillMaxSize()) {
                Box(Modifier.weight((result.green / result.range).toFloat()).fillMaxHeight().background(Color(0xFF008000)))
                Box(Modifier.weight(((result.yellow - result.green) / result.range).toFloat()).fillMaxHeight().background(Color.Yellow))
                Box(Modifier.weight(((result.range - result.yellow) / result.range).toFloat()).fillMaxHeight().background(Color.Red))
            }
            val dot = 11.dp
            Box(
                Modifier
                    .offset(x = maxWidth * result.position.toFloat() - dot / 2, y = (16.dp - dot) / 2)
                    .size(dot)
                    .background(Color.White, CircleShape)
                    .border(1.dp, Color.Black, CircleShape),
            )
        }
    }
}
