package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.Qgc
import one.aircast.mapspike.optText
import org.json.JSONObject
import java.util.Locale

internal const val GPS_RTK_VIEW = "view.gpsRtk"
private const val RTK_POLL_MS = 1000L

internal data class RtkStatus(
    val active: Boolean,
    val valid: Boolean,
    val satellites: Int?,
    val durationS: Double?,
    val accuracyM: Double?,
    val accuracyText: String? = null,
    val latitude: Double? = null,
    val longitude: Double? = null,
    val altitudeM: Double? = null,
)

internal const val RTK_SETTINGS_PAGE = "RTK GPS"
private const val RTK_SETTINGS = "settings.rtkSettings"
private const val RTK_AUTOCONNECT_GROUP = "settings.autoConnectSettings"
private const val RTK_AUTOCONNECT = "autoConnectRTKGPS"

internal fun basePositionWrites(status: RtkStatus): List<Pair<String, Double>>? =
    status.takeIf { it.valid }?.let {
        listOf(
            "fixedBasePositionLatitude" to it.latitude,
            "fixedBasePositionLongitude" to it.longitude,
            "fixedBasePositionAltitude" to it.altitudeM,
            "fixedBasePositionAccuracy" to it.accuracyM,
        ).map { (name, value) -> "$RTK_SETTINGS.$name" to (value ?: return null) }
    }

internal fun fixedBaseChosen(facts: List<Fact>): Boolean =
    facts.find { it.name == "useFixedBasePosition" }?.value.let { it == true || (it as? Number)?.toInt() == 1 || it == "1" }

private fun JSONObject.number(key: String): Double? = if (isNull(key)) null else optDouble(key).takeIf { !it.isNaN() }

internal fun rtkStatus(view: JSONObject?): RtkStatus? =
    view?.takeIf { it.optBoolean("connected") }?.let {
        RtkStatus(
            active = it.optBoolean("active"),
            valid = it.optBoolean("valid"),
            satellites = it.number("numSatellites")?.toInt(),
            durationS = it.number("currentDuration"),
            accuracyM = it.number("currentAccuracy"),
            accuracyText = it.optText("currentAccuracyText").ifEmpty { null },
            latitude = it.number("currentLatitude"),
            longitude = it.number("currentLongitude"),
            altitudeM = it.number("currentAltitude"),
        )
    }

internal fun rtkRows(status: RtkStatus): List<Pair<String, String>> =
    listOfNotNull(
        "Satellites" to (status.satellites?.toString() ?: ""),
        "Duration" to "${String.format(Locale.US, "%.0f", status.durationS ?: 0.0)} s",
        status.accuracyText?.takeIf { (status.accuracyM ?: 0.0) > 0 }?.let { (if (status.valid) "Accuracy" else "Current Accuracy") to it },
    )

internal fun rtkHeadline(status: RtkStatus): String = if (status.active) "Survey-in Active" else "RTK Streaming"

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun RtkIndicatorCell() {
    var status by remember { mutableStateOf<RtkStatus?>(null) }
    var open by remember { mutableStateOf(false) }

    LaunchedEffect(Unit) {
        while (isActive) {
            status = withContext(Dispatchers.Default) { rtkStatus(Qgc.get(GPS_RTK_VIEW)) }
            delay(RTK_POLL_MS)
        }
    }

    val shown = status ?: return
    Text("RTK", style = MaterialTheme.typography.labelMedium, modifier = Modifier.clickable { open = true })

    if (open) {
        ModalBottomSheet(onDismissRequest = { open = false }) {
            Column(
                Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = 20.dp).padding(bottom = 24.dp),
                verticalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                Text("RTK GPS Status", style = MaterialTheme.typography.titleSmall)
                Text(rtkHeadline(shown), style = MaterialTheme.typography.bodyMedium)
                rtkRows(shown).forEach { (label, value) -> Text("$label  $value", style = MaterialTheme.typography.bodySmall) }
                RtkSettingsSection(shown)
            }
        }
    }
}

@Composable
private fun RtkSettingsSection(status: RtkStatus) {
    val scope = rememberCoroutineScope()
    var reloads by remember { mutableIntStateOf(0) }
    var facts by remember { mutableStateOf(emptyList<Fact>()) }
    var autoConnect by remember { mutableStateOf<Fact?>(null) }

    LaunchedEffect(reloads) {
        withContext(Dispatchers.Default) {
            facts = settingsSections(Qgc.get(settingsPagePath(RTK_SETTINGS_PAGE))).flatMap { section -> section.blocks.flatMap { it.facts } }
            autoConnect = Qgc.get("$RTK_AUTOCONNECT_GROUP.$RTK_AUTOCONNECT")?.let { Qgc.fact(RTK_AUTOCONNECT_GROUP, it.put("name", RTK_AUTOCONNECT)) }
        }
    }

    Text("RTK GPS Settings", style = MaterialTheme.typography.titleSmall)
    autoConnect?.let { FactRow(it, title = "AutoConnect") { reloads++ } }
    facts.forEach { FactRow(it) { reloads++ } }
    val writes = basePositionWrites(status)
    if (fixedBaseChosen(facts)) Row(verticalAlignment = Alignment.CenterVertically) {
        Text("Current base position", modifier = Modifier.weight(1f))
        OutlinedButton(enabled = writes != null, onClick = {
            scope.launch {
                withContext(Dispatchers.Default) { writes?.forEach { (path, value) -> Qgc.set(path, value) } }
                reloads++
            }
        }) { Text(if (writes != null) "Save" else "Not Yet Valid") }
    }
}
