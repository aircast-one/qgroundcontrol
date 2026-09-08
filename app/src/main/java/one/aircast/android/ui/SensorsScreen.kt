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
import one.aircast.android.bridge.qgcBool
import one.aircast.android.bridge.qgcDouble
import one.aircast.android.bridge.qgcString

private const val CAL = "sensorsCal"

internal data class Calibration(
    val name: String,
    val method: String,
    val instruction: String,
    val warning: String = "",
    // APMSensorsComponent.qml blocks these two while the accelerometer needs calibrating.
    // A compass calibration against an uncalibrated accelerometer produces a result the
    // operator has no reason to distrust, which is worse than refusing to start it. The
    // accelerometer itself is never blocked, because it is the way out.
    val needsAccelFirst: Boolean = false,
)

internal fun blockedByAccel(calibration: Calibration, accelNeeded: Boolean): Boolean =
    calibration.needsAccelFirst && accelNeeded

private const val CAL_START_MS = 5000L

// Every calibration entry point on APMSensorsComponentController is void, so the bridge
// can only say the method was called. The dialog closes either way, and a start that did
// nothing leaves the operator back at the list with no calibration and nothing said.
internal fun calibrationFailure(name: String, started: Boolean): String? =
    if (started) null else "$name calibration did not start."

private fun calibrationRunning(): Boolean =
    Qgc.get("$CAL.calibrationInProgress").opt("value") == true

private fun calibrationStatus(): String =
    Qgc.get("$CAL.statusText").opt("value")?.toString().orEmpty()

// calibrationInProgress alone is not enough: pressure finishes before a 150 ms poll can
// see it, so watching only that flag reported a calibration that had already succeeded as
// one that never started. The controller writes "Requesting ..." to statusText as it
// begins, so a changed status is the evidence a fast routine leaves behind.
internal fun calibrationBegan(running: Boolean, statusBefore: String, statusNow: String): Boolean =
    running || statusNow != statusBefore

internal val CALIBRATIONS = listOf(
    Calibration(
        name = "Accelerometer",
        method = "calibrateAccel",
        instruction = "You will be asked to hold the vehicle still in six orientations. " +
            "Press Next once it is steady in each one.",
    ),
    Calibration(
        name = "Compass",
        method = "calibrateCompass",
        needsAccelFirst = true,
        instruction = "Rotate the vehicle slowly around all axes until the bar fills. " +
            "Stand away from metal, cars and reinforced concrete.",
    ),
    Calibration(
        name = "Level Horizon",
        method = "levelHorizon",
        needsAccelFirst = true,
        instruction = "Place the vehicle in its level flight position and leave it still.",
        warning = "Sets what the vehicle considers level. Get this wrong and it will drift in flight.",
    ),
    Calibration(
        name = "Gyro",
        method = "calibrateGyro",
        instruction = "Place the vehicle on a surface and leave it completely still.",
    ),
    Calibration(
        name = "Pressure",
        method = "calibratePressure",
        instruction = "Zeroes the altitude at the current pressure. Do this where you will take off.",
    ),
)

private val ORIENTATIONS = listOf(
    "Down" to "Level",
    "UpsideDown" to "Upside down",
    "Left" to "Left side",
    "Right" to "Right side",
    "NoseDown" to "Nose down",
    "TailDown" to "Tail down",
)

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
    calibration: Calibration,
    onConfirm: () -> Unit,
    onDismiss: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Calibrate ${calibration.name}?") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Text(calibration.instruction)
                if (calibration.warning.isNotBlank()) {
                    Text(
                        text = calibration.warning,
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
private fun OrientationGrid() {
    val visible = ORIENTATIONS.map { (key, _) -> qgcBool("$CAL.orientationCal${key}SideVisible").value }
    val done = ORIENTATIONS.map { (key, _) -> qgcBool("$CAL.orientationCal${key}SideDone").value }
    val inProgress = ORIENTATIONS.map { (key, _) -> qgcBool("$CAL.orientationCal${key}SideInProgress").value }
    val rotate = ORIENTATIONS.map { (key, _) -> qgcBool("$CAL.orientationCal${key}SideRotate").value }

    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        ORIENTATIONS.indices.filter { visible[it] }.chunked(2).forEach { row ->
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                row.forEach { index ->
                    Box(Modifier.weight(1f)) {
                        OrientationTile(
                            label = ORIENTATIONS[index].second,
                            done = done[index],
                            inProgress = inProgress[index],
                            rotate = rotate[index],
                        )
                    }
                }
                repeat(2 - row.size) { Box(Modifier.weight(1f)) {} }
            }
        }
    }
}

@Composable
private fun RunningCalibration(name: String, modifier: Modifier = Modifier) {
    val progress by qgcDouble("$CAL.calProgress", 0.0)
    val helpText by qgcString("$CAL.orientationHelpText")
    val statusText by qgcString("$CAL.statusText")
    val nextEnabled by qgcBool("$CAL.nextEnabled")
    val cancelEnabled by qgcBool("$CAL.cancelEnabled")
    val showOrientations by qgcBool("$CAL.showOrientationCalArea")

    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Text("Calibrating $name", style = MaterialTheme.typography.titleMedium)

        LinearProgressIndicator(
            progress = { progress.toFloat().coerceIn(0f, 1f) },
            modifier = Modifier.fillMaxWidth(),
        )

        if (helpText.isNotBlank()) {
            Text(helpText, style = MaterialTheme.typography.bodyMedium)
        }

        if (showOrientations) {
            OrientationGrid()
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
                enabled = nextEnabled,
                modifier = Modifier.weight(1f),
            ) { Text("Next") }
            OutlinedButton(
                onClick = { offMainDetached { Qgc.invoke("$CAL.cancelCalibration") } },
                enabled = cancelEnabled,
                modifier = Modifier.weight(1f),
            ) { Text("Cancel") }
        }
    }
}

@Composable
fun SensorsScreen(modifier: Modifier = Modifier) {
    val hasVehicle by qgcBool("vehicles.activeVehicleAvailable")
    val isPx4 by qgcBool("vehicle.px4Firmware")
    val running by qgcBool("$CAL.calibrationInProgress")
    val compassNeeded by qgcBool("$CAL.compassSetupNeeded")
    val accelNeeded by qgcBool("$CAL.accelSetupNeeded")
    val lastResult by qgcString("$CAL.statusText")
    var pending by remember { mutableStateOf<Calibration?>(null) }
    var runningName by remember { mutableStateOf("") }
    var notice by remember { mutableStateOf<String?>(null) }
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

    pending?.let { calibration ->
        StartDialog(
            calibration = calibration,
            onConfirm = {
                runningName = calibration.name
                notice = null
                scope.launch {
                    val before = withContext(Dispatchers.Default) { calibrationStatus() }
                    val dispatched = withContext(Dispatchers.Default) {
                        if (calibration.method == "calibrateAccel") {
                            Qgc.invoke("$CAL.calibrateAccel", false)
                        } else {
                            Qgc.invoke("$CAL.${calibration.method}")
                        }
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
                    notice = calibrationFailure(calibration.name, started)
                }
            },
            onDismiss = { pending = null },
        )
    }

    if (running) {
        RunningCalibration(runningName, modifier)
        return
    }

    LazyColumn(
        modifier = modifier.fillMaxSize(),
    ) {
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

        items(CALIBRATIONS, key = { it.name }) { calibration ->
            val needed = when (calibration.name) {
                "Accelerometer" -> accelNeeded
                "Compass" -> compassNeeded
                else -> null
            }
            val blocked = blockedByAccel(calibration, accelNeeded)
            SetupRow(
                title = calibration.name,
                status = when {
                    blocked -> "Calibrate the accelerometer first"
                    needed == true -> "Not calibrated"
                    needed == false -> "Calibrated"
                    else -> ""
                },
                state = when {
                    blocked -> SetupState.Neutral
                    needed == true -> SetupState.NeedsAttention
                    needed == false -> SetupState.Done
                    else -> SetupState.Neutral
                },
                onClick = if (blocked) null else ({ pending = calibration }),
            )
        }

        if (lastResult.isNotBlank()) {
            item(key = "last") {
                SectionHeader("Last calibration")
                Text(
                    text = lastResult,
                    style = MaterialTheme.typography.bodyMedium,
                    fontFamily = FontFamily.Monospace,
                    modifier = Modifier.padding(horizontal = 20.dp),
                )
            }
        }

        item(key = "footnote") {
            FootNote(
                "Calibrate where the aircraft will fly, away from metal, with the " +
                    "propellers off. CompassMot and the motor test stay on the desktop: " +
                    "they spin the propellers and need someone watching the aircraft.",
            )
        }
    }
}
