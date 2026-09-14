package one.aircast.android.ui

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.input.KeyboardType
import kotlin.math.abs
import kotlin.math.floor
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcFacts

data class SettingsGroup(val path: String, val title: String, val description: String)

const val LINKS_GROUP_PATH = "links"
const val UNITS_GROUP_PATH = "settings.unitsSettings"
const val VIDEO_GROUP_PATH = "settings.videoSettings"
const val FLY_VIEW_GROUP_PATH = "settings.flyViewSettings"

val SETTINGS_GROUPS = listOf(
    SettingsGroup(LINKS_GROUP_PATH, "Comm Links", "Serial, UDP and TCP connections to the vehicle"),
    SettingsGroup(UNITS_GROUP_PATH, "Units", "Metric or imperial, or each measurement chosen separately"),
    SettingsGroup(VIDEO_GROUP_PATH, "Video", "Stream source and address, and the cameras to switch between"),
    SettingsGroup(FLY_VIEW_GROUP_PATH, "Fly View", "What the flight screen shows, and the on-screen RC controls"),
    SettingsGroup("settings.planViewSettings", "Plan View", "Defaults and rules for building a mission"),
    SettingsGroup("settings.mapsSettings", "Maps", "How much map imagery is kept on this device"),
    SettingsGroup("settings.flightMapSettings", "Flight Map", "Which provider draws the map under the aircraft"),
    SettingsGroup("settings.offlineMapsSettings", "Offline Maps", "Zoom range and tile budget when downloading for a flight"),
    SettingsGroup("settings.gimbalControllerSettings", "Gimbal", "On-screen gimbal control and the camera's field of view"),
    SettingsGroup("settings.apmMavlinkStreamRateSettings", "Stream rates", "How often the vehicle is asked to send each kind of telemetry"),
    SettingsGroup("settings.batteryIndicatorSettings", "Battery", "What the battery indicator shows, and when it warns"),
    SettingsGroup("settings.autoConnectSettings", "AutoConnect", "Which link types connect on their own"),
    SettingsGroup("settings.mavlinkSettings", "MAVLink and telemetry logs", "Telemetry logging, stream requests and forwarding"),
    SettingsGroup("settings.rtkSettings", "RTK GPS", "Base station accuracy and position"),
    SettingsGroup("settings.adsbVehicleManagerSettings", "ADSB Traffic", "The SBS-1 receiver the traffic readout draws from"),
    SettingsGroup("settings.remoteIDSettings", "Remote ID", "Operator and aircraft identification, which some regions require in flight"),
    SettingsGroup("settings.appSettings", "General", "Offline editing defaults and other app-wide settings"),
)

@Composable
fun SettingsScreen(modifier: Modifier = Modifier) {
    var group by remember { mutableStateOf<SettingsGroup?>(null) }

    BackHandler(enabled = group != null) { group = null }

    val current = group
    if (current == null) {
        LazyColumn(modifier.fillMaxSize()) {
            items(SETTINGS_GROUPS) { entry ->
                ListItem(
                    headlineContent = { Text(entry.title) },
                    supportingContent = { Text(entry.description) },
                    modifier = Modifier.clickable { group = entry },
                )
                HorizontalDivider()
            }
        }
        return
    }

    Column(modifier.fillMaxSize()) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            IconButton(onClick = { group = null }) { Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back") }
            Text(current.title, style = MaterialTheme.typography.titleLarge)
        }
        if (current.path == LINKS_GROUP_PATH) {
            LinksScreen(Modifier.fillMaxSize())
        } else if (current.path == UNITS_GROUP_PATH) {
            UnitsPage(Modifier.fillMaxSize())
        } else if (current.path == VIDEO_GROUP_PATH) {
            FactList(current.path, Modifier.fillMaxSize()) { ExtraVideoSourcesEditor() }
        } else if (current.path == FLY_VIEW_GROUP_PATH) {
            FactList(current.path, Modifier.fillMaxSize()) { RcControlsEditor() }
        } else {
            FactList(current.path, Modifier.fillMaxSize())
        }
    }
}

@Composable
fun FactList(groupPath: String, modifier: Modifier = Modifier, footer: @Composable () -> Unit = {}) {
    val facts by qgcFacts(groupPath)

    LazyColumn(modifier) {
        if (facts.isEmpty()) {
            item(key = "empty") { Text("No settings exposed here.", Modifier.padding(16.dp)) }
        }
        sectionedFacts(groupPath, facts).forEach { (title, members) ->
            if (title.isNotBlank()) {
                item(key = "head$title") { SectionHeader(title) }
            }
            items(members, key = { it.path }) { fact ->
                FactRow(fact)
                HorizontalDivider()
            }
        }
        item(key = "footer") { footer() }
    }
}

internal fun enumLabel(fact: Fact): String =
    fact.enumStrings.getOrNull(fact.enumIndex) ?: fact.valueString

internal fun factSubtitle(fact: Fact): String = when {
    fact.enumStrings.isNotEmpty() || fact.bitmaskStrings.isNotEmpty() || fact.isBool -> ""
    else -> fact.units
}

@Composable
internal fun FactRow(
    fact: Fact,
    title: String = fact.title,
    subtitle: String = factSubtitle(fact),
    onWrite: () -> Unit = {},
) {
    val scope = rememberCoroutineScope()
    var refusal by remember(fact.path) { mutableStateOf<String?>(null) }

    fun write(block: () -> Boolean) {
        scope.launch {
            val accepted = withContext(Dispatchers.Default) { block() }
            refusal = writeRefusal(accepted)
            if (accepted) onWrite()
        }
    }

    Column {
    Row(
        Modifier
            .fillMaxWidth()
            .heightIn(min = 64.dp)
            .padding(horizontal = 20.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Column(Modifier.weight(1f)) {
            Text(
                text = title,
                style = MaterialTheme.typography.bodyLarge,
                maxLines = 3,
                overflow = TextOverflow.Ellipsis,
            )
            if (subtitle.isNotBlank()) {
                Text(
                    text = subtitle,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }

        Box(Modifier.widthIn(max = 190.dp), contentAlignment = Alignment.CenterEnd) {
            when {
                !controlIsUnderstood(fact.controlKind) -> Column(
                    horizontalAlignment = Alignment.End,
                ) {
                    Text(
                        text = enumLabel(fact),
                        style = MaterialTheme.typography.bodyMedium,
                        maxLines = 2,
                        overflow = TextOverflow.Ellipsis,
                    )
                    Text(
                        text = "Edit on desktop",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                fact.readOnly -> Column(horizontalAlignment = Alignment.End) {
                    Text(
                        text = enumLabel(fact),
                        style = MaterialTheme.typography.bodyMedium,
                        maxLines = 2,
                        overflow = TextOverflow.Ellipsis,
                    )
                    Text(
                        text = "Read-only",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                fact.isBool -> Switch(
                    checked = fact.boolValue,
                    onCheckedChange = { checked -> write { Qgc.set(fact.path, checked) } },
                )
                fact.isBitmask -> BitmaskPicker(fact, ::write)
                fact.isEnum && !fact.valueIsOffTheEnumList -> EnumPicker(fact, ::write)
                else -> FactTextField(fact, onWrite)
            }
        }
    }
    refusal?.let {
        Text(
            text = it,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.error,
            modifier = Modifier.padding(start = 20.dp, bottom = 8.dp),
        )
    }
    }
}

@Composable
private fun BitmaskPicker(fact: Fact, write: (() -> Boolean) -> Unit) {
    var editing by remember(fact.path) { mutableStateOf(false) }
    val raw = bitmaskRaw(fact)

    TextButton(
        onClick = { editing = true },
        contentPadding = PaddingValues(horizontal = 4.dp, vertical = 8.dp),
    ) {
        Text(
            text = bitmaskSummary(fact),
            style = MaterialTheme.typography.bodyMedium,
            maxLines = 3,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.weight(1f, fill = false),
        )
        Icon(
            imageVector = Icons.Default.KeyboardArrowDown,
            contentDescription = null,
            modifier = Modifier.padding(start = 2.dp),
        )
    }

    if (editing) {
        AlertDialog(
            onDismissRequest = { editing = false },
            title = { Text(fact.title) },
            text = {
                Column(Modifier.verticalScroll(rememberScrollState())) {
                    fact.bitmaskStrings.indices.forEach { index ->
                        val bit = fact.bitmaskValues[index]
                        Row(
                            Modifier
                                .fillMaxWidth()
                                .clickable {
                                    write { Qgc.set(fact.path, (raw xor bit).toString()) }
                                }
                                .padding(vertical = 6.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(12.dp),
                        ) {
                            Checkbox(
                                checked = raw and bit != 0L,
                                onCheckedChange = {
                                    write { Qgc.set(fact.path, (raw xor bit).toString()) }
                                },
                            )
                            Text(fact.bitmaskStrings[index], Modifier.weight(1f))
                        }
                    }
                }
            },
            confirmButton = { TextButton(onClick = { editing = false }) { Text("Done") } },
        )
    }
}

@Composable
private fun EnumPicker(fact: Fact, write: (() -> Boolean) -> Unit) {
    var expanded by remember { mutableStateOf(false) }
    val label = enumLabel(fact)

    Column {
        TextButton(
            onClick = { expanded = true },
            contentPadding = PaddingValues(horizontal = 4.dp, vertical = 8.dp),
        ) {
            Text(
                text = label,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.weight(1f, fill = false),
            )
            Icon(
                imageVector = Icons.Default.KeyboardArrowDown,
                contentDescription = null,
                modifier = Modifier.padding(start = 2.dp),
            )
        }
        DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            fact.enumStrings.forEachIndexed { index, option ->
                DropdownMenuItem(
                    text = { Text(option) },
                    onClick = {
                        expanded = false
                        write { Qgc.set("${fact.path}.enumIndex", index) }
                    },
                )
            }
        }
    }
}

internal fun factKeyboard(fact: Fact): KeyboardType = when {
    fact.isString || fact.isBool -> KeyboardType.Text
    fact.minString.toDoubleOrNull()?.let { it < 0.0 } != false -> KeyboardType.Text
    else -> KeyboardType.Decimal
}

internal fun plainNumber(text: String): String {
    val parsed = text.toDoubleOrNull() ?: return text
    if (!parsed.isFinite() || abs(parsed) >= 1e15) return text
    return when (parsed == floor(parsed)) {
        true -> parsed.toLong().toString()
        false -> parsed.toBigDecimal().stripTrailingZeros().toPlainString()
    }
}

internal fun factConstraintNote(fact: Fact): String? {
    val parts = listOfNotNull(
        fact.minString.takeIf { it.isNotBlank() && !fact.minIsDefaultForType }?.let { "Min ${plainNumber(it)}" },
        fact.maxString.takeIf { it.isNotBlank() && !fact.maxIsDefaultForType }?.let { "Max ${plainNumber(it)}" },
        fact.defaultValueString.takeIf { it.isNotBlank() }?.let { "Default ${plainNumber(it)}" },
    )
    return parts.takeIf { it.isNotEmpty() }?.joinToString(" · ")
}

internal fun factRebootNote(fact: Fact): String? = when {
    fact.vehicleRebootRequired -> "Reboot the vehicle for this to take effect."
    fact.qgcRebootRequired -> "Restart Aircast for this to take effect."
    else -> null
}

// Qgc.set returns false when the bridge refuses the write, and the reason it carries
// names a property and a class — true, and no use to a pilot. So the row says only what
// is known: the change did not take. Without this a refused switch flips back on the
// next poll and reads as the app glitching.
internal fun writeRefusal(accepted: Boolean): String? =
    if (accepted) null else "That change was not accepted."

internal fun validationMessage(result: Any?): String? =
    (result as? String)?.takeIf { it.isNotBlank() }

private suspend fun rejectionFor(fact: Fact, text: String): String? =
    withContext(Dispatchers.Default) {
        validationMessage(Qgc.invokeResult("${fact.path}.validate", text, false))
    }

@Composable
private fun FactTextField(fact: Fact, onWrite: () -> Unit) {
    var editing by remember(fact.path) { mutableStateOf<String?>(null) }
    var rejection by remember(fact.path) { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    Column {
        OutlinedTextField(
            value = editing ?: fact.valueString,
            onValueChange = {
                editing = it
                rejection = null
            },
            singleLine = true,
            isError = rejection != null,
            keyboardOptions = KeyboardOptions(keyboardType = factKeyboard(fact)),
            modifier = Modifier.fillMaxWidth(),
            trailingIcon = {
                val committed = editing
                if (committed != null && committed != fact.valueString) {
                    TextButton(onClick = {
                        scope.launch {
                            val refused = rejectionFor(fact, committed)
                            if (refused != null) {
                                rejection = refused
                                return@launch
                            }
                            // Validation only says the value is well formed. The bridge can
                            // still refuse the write, and the fact then reads back unchanged,
                            // which is indistinguishable from a value accepted as-is.
                            val accepted = withContext(Dispatchers.Default) {
                                Qgc.set(fact.path, committed)
                            }
                            rejection = writeRefusal(accepted)
                            if (accepted) {
                                editing = null
                                onWrite()
                            }
                        }
                    }) { Text("Set") }
                }
            },
        )
        rejection?.let {
            Text(
                text = it,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
            )
        }
        if (rejection == null) {
            factConstraintNote(fact)?.let {
                Text(
                    text = it,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
        factRebootNote(fact)?.let {
            Text(
                text = it,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.tertiary,
            )
        }
    }
}
