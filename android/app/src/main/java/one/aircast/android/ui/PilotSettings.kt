package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.produceState
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.qgcPath

internal data class PilotSetting(val label: String, val parameters: List<String>, val section: String, val hint: String, val whenUnlimited: String? = null, val quantity: String? = null)

private const val RETURN_HOME = "Return to home"
private const val FLIGHT_PROTECTION = "Flight protection"
private const val FAILSAFES = "If something goes wrong"
private const val FLIGHT_LIMITS = "Flight limits"
private const val ALTITUDE = "altitude"
private const val DISTANCE = "distance"
private const val SPEED = "speed"

internal fun pilotSettings(group: SettingsGroup): List<PilotSetting> = when (group) {
    SettingsGroup.Safety -> listOf(
        PilotSetting("Return-to-home altitude", listOf("RTL_RETURN_ALT", "RTL_ALT_M", "RTL_ALT"), RETURN_HOME, "Climbs to this height before flying home.", quantity = ALTITUDE),
        PilotSetting("Max altitude", listOf("GF_MAX_VER_DIST", "FENCE_ALT_MAX"), FLIGHT_PROTECTION, "The aircraft will not climb above this.", "Most countries cap flights at 120 m.", ALTITUDE),
        PilotSetting("Max distance", listOf("GF_MAX_HOR_DIST", "FENCE_RADIUS"), FLIGHT_PROTECTION, "The aircraft will not fly farther from home than this.", quantity = DISTANCE),
        PilotSetting("Signal lost", listOf("NAV_DLL_ACT", "FS_GCS_ENABLE"), FAILSAFES, "When the link to this app drops."),
        PilotSetting("Remote controller lost", listOf("NAV_RCL_ACT", "FS_THR_ENABLE"), FAILSAFES, "When the remote controller drops."),
        PilotSetting("Low battery", listOf("COM_LOW_BAT_ACT", "BATT_FS_LOW_ACT"), FAILSAFES, "When the battery runs low."),
    )
    SettingsGroup.Control -> listOf(
        PilotSetting("Max horizontal speed", listOf("MPC_XY_VEL_MAX", "LOIT_SPEED_MS", "LOIT_SPEED"), FLIGHT_LIMITS, "Fastest the aircraft flies forward and sideways.", quantity = SPEED),
        PilotSetting("Max climb speed", listOf("MPC_Z_VEL_MAX_UP", "PILOT_SPD_UP", "PILOT_SPEED_UP"), FLIGHT_LIMITS, "Fastest the aircraft climbs.", quantity = SPEED),
        PilotSetting("Max descent speed", listOf("MPC_Z_VEL_MAX_DN", "PILOT_SPD_DN", "PILOT_SPEED_DN"), FLIGHT_LIMITS, "Fastest the aircraft descends.", "At 0 it descends as fast as it climbs.", SPEED),
    )
    else -> emptyList()
}

internal fun pilotSearchHits(query: String): List<PilotSetting> =
    query.trim().lowercase().takeIf { it.isNotEmpty() }?.let { wanted ->
        SettingsGroup.entries
            .flatMap { group -> pilotSettings(group).map { it.copy(section = "${group.title} \u203a ${it.section}") } }
            .filter { setting -> (listOf(setting.label, setting.section) + setting.parameters).any { it.lowercase().contains(wanted) } }
    }.orEmpty()

internal data class ShownPilotSetting(val setting: PilotSetting, val fact: Fact)

internal fun firstReported(setting: PilotSetting, reported: (String) -> Fact?): Fact? =
    setting.parameters.firstNotNullOfOrNull(reported)?.copy(shortLabel = setting.label)

@Composable
internal fun PilotSettings(group: SettingsGroup) = PilotSettings(pilotSettings(group))

@Composable
internal fun PilotSettings(settings: List<PilotSetting>) {
    if (settings.isEmpty()) return
    if (!hasVehicle()) return OfflinePilotSettings(settings, OFFLINE_PILOT_NOTE, offersLink = true)
    val setup by qgcPath(SETUP)
    if (!parametersReady(setup)) return OfflinePilotSettings(settings, LOADING_PILOT_NOTE, offersLink = false)
    val shown by produceState(emptyList<ShownPilotSetting>(), settings, setup) {
        value = withContext(Dispatchers.Default) { settings.mapNotNull { setting -> firstReported(setting) { parameterFact(it, setting.quantity) }?.let { ShownPilotSetting(setting, it) } } }
    }
    shown.groupBy { it.setting.section }.map { (section, rows) ->
        SectionHeader(section)
        rows.map { PilotFactRow(it.fact, it.setting) }
    }
}

internal const val OFFLINE_PILOT_NOTE = "Connect the aircraft to see and change these."
internal const val LOADING_PILOT_NOTE = "Loading these from the aircraft\u2026"
private const val OFFLINE_VALUE = "\u2014"
private const val OFFLINE_ALPHA = 0.38f

internal fun pilotSections(settings: List<PilotSetting>): Map<String, List<PilotSetting>> = settings.groupBy { it.section }

@Composable
private fun OfflinePilotSettings(settings: List<PilotSetting>, note: String, offersLink: Boolean) {
    val navigation = LocalAppNavigation.current
    Row(Modifier.fillMaxWidth().heightIn(min = 48.dp).padding(start = 16.dp, end = 8.dp, top = 12.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(note, Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        if (offersLink) TextButton(onClick = { navigation.settingsPage = CONNECTIONS_PAGE }) { Text("Add a link") }
    }
    pilotSections(settings).map { (section, rows) ->
        SectionHeader(section)
        rows.map { setting -> OfflineRow(setting.label, setting.hint) }
    }
}

@Composable
private fun OfflineRow(label: String, hint: String?) {
    Row(
        Modifier.fillMaxWidth().heightIn(min = 56.dp).padding(horizontal = 16.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        androidx.compose.foundation.layout.Column(Modifier.weight(1f)) {
            Text(label, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium, color = MaterialTheme.colorScheme.onSurface.copy(alpha = OFFLINE_ALPHA))
            hint?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = OFFLINE_ALPHA)) }
        }
        Text(OFFLINE_VALUE, style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = OFFLINE_ALPHA))
    }
}

private val OFFLINE_SENSORS = listOf("Compass", "Accelerometer", "Gyroscope")

private val SENSOR_CHECKS = listOf("compass", "accelerometer", "gyro")
private const val SENSORS_SECTION = "Sensors"
private const val CALIBRATED = "Calibrated"

internal fun sensorChecks(state: CalibrationState?): List<CalibrationRoutine> =
    SENSOR_CHECKS.mapNotNull { id -> state?.routines?.firstOrNull { it.id == id } }

internal fun sensorHealthy(routine: CalibrationRoutine): Boolean = routine.status == CALIBRATED

@Composable
internal fun SensorChecks(onCalibrate: () -> Unit) {
    val json by qgcPath(CALIBRATION)
    if (!hasVehicle()) {
        SectionHeader(SENSORS_SECTION)
        OFFLINE_SENSORS.map { OfflineRow(it, null) }
        return
    }
    val checks = sensorChecks(calibrationState(json))
    if (checks.isEmpty()) return
    SectionHeader(SENSORS_SECTION)
    checks.map { routine ->
        Row(
            Modifier.fillMaxWidth().heightIn(min = 56.dp).padding(start = 16.dp, end = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Text(sentenceCase(routine.title), Modifier.weight(1f), style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
            Text(
                if (sensorHealthy(routine)) "Normal" else routine.status.ifBlank { "Not checked" },
                style = MaterialTheme.typography.bodyMedium,
                color = if (sensorHealthy(routine)) MaterialTheme.colorScheme.onSurfaceVariant else MaterialTheme.colorScheme.error,
            )
            if (!sensorHealthy(routine)) TextButton(onClick = onCalibrate, enabled = routine.enabled) { Text("Calibrate") }
        }
    }
    SetupRow(title = "Calibrate sensors", status = "", onClick = onCalibrate)
}

@Composable
private fun PilotFactRow(shown: Fact, setting: PilotSetting) {
    val name = shown.name
    var revision by remember { mutableIntStateOf(0) }
    val live by qgcPath(parameterPath(name, setting.quantity))
    val fact by produceState(shown, name, live, revision) {
        value = withContext(Dispatchers.Default) { parameterFact(name, setting.quantity)?.copy(shortLabel = shown.shortLabel) } ?: shown
    }
    FactRow(pilotChoices(fact), subtitle = setting.hint, warning = setting.whenUnlimited?.takeIf { factNumber(fact) == 0f }, onWrite = { revision++ })
}

private val PILOT_CHOICES = mapOf(
    "Disabled" to "Do nothing",
    "Hold mode" to "Hover",
    "Loiter mode" to "Hover",
    "Return mode" to "Return home",
    "Land mode" to "Land",
    "Warning" to "Warn only",
    "Return at critical level, land at emergency level" to "Return home, land if critical",
    "Terminate" to "Stop motors",
)

internal fun pilotChoice(label: String): String = PILOT_CHOICES[label] ?: label

internal fun pilotChoices(fact: Fact): Fact = fact.copy(enumStrings = fact.enumStrings.map(::pilotChoice))

private val PILOT_LABELS = mapOf(
    "guidedMinimumAltitude" to "Lowest altitude for takeoff and fly-to",
    "guidedMaximumAltitude" to "Highest altitude for takeoff and fly-to",
    "maxGoToLocationDistance" to "Max fly-to distance",
    "forwardFlightGoToLocationLoiterRad" to "Circle radius after fly-to (planes)",
    "goToLocationRequiresConfirmInGuided" to "Confirm fly-to first",
    "updateHomePosition" to "Home follows this phone",
    "adsbServerConnectEnabled" to "Show nearby aircraft (ADS-B)",
    "adsbServerHostAddress" to "ADS-B server address",
    "adsbServerPort" to "ADS-B server port",
    "displayPresetsTabFirst" to "Open Plan on templates",
    "useConditionGate" to "Use gate commands in survey patterns",
    "takeoffItemNotRequired" to "Missions can start without a takeoff",
    "allowMultipleLandingPatterns" to "Allow several landing paths",
    "vtolTransitionDistance" to "VTOL transition distance",
    "keepMapCenteredOnVehicle" to "Keep the aircraft centred on the map",
    "showAdditionalIndicatorsCompass" to "Extra compass markers",
    "lockNoseUpCompass" to "North-up compass",
    "showObstacleDistanceOverlay" to "Show obstacle distances",
    "showPhotoVideoControl" to "Show the shutter button",
    "showSimpleCameraControl" to "Simple camera trigger",
    "showLogReplayStatusBar" to "Show the log replay bar",
    "requestControlAllowTakeover" to "Let other stations take control",
    "requestControlTimeout" to "Control request timeout",
    "valueDisplay" to "Battery shows",
    "threshold1" to "Battery warning level",
    "threshold2" to "Battery critical level",
    "consolidateMultipleBatteries" to "Combine multiple batteries",
    "streamEnabled" to "Show video",
    "videoFit" to "Video fit",
    "gridLines" to "Grid lines",
    "recordingFormat" to "Recording format",
    "maxVideoSize" to "Storage limit",
    "rtspTimeout" to "Connection timeout",
    "forceVideoDecoder" to "Video decoder",
    "aspectRatio" to "Aspect ratio",
    "defaultMissionItemAltitude" to "Default waypoint altitude",
    "offlineEditingFirmwareClass" to "Plan offline for firmware",
    "offlineEditingVehicleClass" to "Plan offline for aircraft type",
    "offlineEditingCruiseSpeed" to "Planning cruise speed",
    "offlineEditingHoverSpeed" to "Planning hover speed",
    "offlineEditingAscentSpeed" to "Planning climb speed",
    "offlineEditingDescentSpeed" to "Planning descent speed",
    "followTarget" to "Send this phone's location (Follow Me)",
    "enableMultiVehiclePanel" to "Show the multi-aircraft panel",
    "androidDontSaveToSDCard" to "Save data to the SD card",
    "disableAllPersistence" to "Don't save any data",
)

internal fun pilotWorded(fact: Fact): Fact =
    PILOT_LABELS[fact.name]?.let { label -> fact.copy(shortLabel = label) } ?: fact
