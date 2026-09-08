package one.aircast.android.ui

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
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
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
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcFacts

data class SettingsGroup(val path: String, val title: String)

const val LINKS_GROUP_PATH = "links"
const val UNITS_GROUP_PATH = "settings.unitsSettings"

val SETTINGS_GROUPS = listOf(
    SettingsGroup(LINKS_GROUP_PATH, "Comm Links"),
    SettingsGroup(UNITS_GROUP_PATH, "Units"),
    SettingsGroup("settings.videoSettings", "Video"),
    SettingsGroup("settings.flyViewSettings", "Fly View"),
    SettingsGroup("settings.planViewSettings", "Plan View"),
    SettingsGroup("settings.mapsSettings", "Maps"),
    SettingsGroup("settings.batteryIndicatorSettings", "Battery"),
    SettingsGroup("settings.autoConnectSettings", "AutoConnect"),
    SettingsGroup("settings.appSettings", "General"),
)

@Composable
fun SettingsScreen(modifier: Modifier = Modifier) {
    var group by remember { mutableStateOf<SettingsGroup?>(null) }

    val current = group
    if (current == null) {
        LazyColumn(modifier.fillMaxSize()) {
            items(SETTINGS_GROUPS) { entry ->
                SetupRow(title = entry.title, onClick = { group = entry })
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
        } else {
            FactList(current.path, Modifier.fillMaxSize())
        }
    }
}

@Composable
fun FactList(groupPath: String, modifier: Modifier = Modifier) {
    val facts by qgcFacts(groupPath)

    if (facts.isEmpty()) {
        Text("No settings exposed here.", modifier.padding(16.dp))
        return
    }

    LazyColumn(modifier) {
        sectionedFacts(groupPath, facts).forEach { (title, members) ->
            if (title.isNotBlank()) {
                item(key = "head$title") { SectionHeader(title) }
            }
            items(members, key = { it.path }) { fact ->
                FactRow(fact)
                HorizontalDivider()
            }
        }
        desktopOnlyNote(facts)?.let { note ->
            item(key = "desktoponly") { FootNote(note) }
        }
    }
}

internal fun enumLabel(fact: Fact): String =
    fact.enumStrings.getOrNull(fact.enumIndex) ?: fact.valueString

@Composable
internal fun FactRow(
    fact: Fact,
    title: String = fact.title,
    subtitle: String = fact.units,
    onWrite: () -> Unit = {},
) {
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
                fact.readOnly -> Text(
                    text = enumLabel(fact),
                    style = MaterialTheme.typography.bodyMedium,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )
                fact.isBool -> Switch(
                    checked = fact.boolValue,
                    onCheckedChange = { checked ->
                        offMainDetached { Qgc.set(fact.path, checked); onWrite() }
                    },
                )
                fact.isEnum && !fact.valueIsOffTheEnumList -> EnumPicker(fact, onWrite)
                else -> FactTextField(fact, onWrite)
            }
        }
    }
}

@Composable
private fun EnumPicker(fact: Fact, onWrite: () -> Unit) {
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
                        offMainDetached {
                            Qgc.set("${fact.path}.enumIndex", index)
                            onWrite()
                        }
                    },
                )
            }
        }
    }
}

internal fun factConstraintNote(fact: Fact): String? {
    val parts = listOfNotNull(
        fact.minString.takeIf { it.isNotBlank() && !fact.minIsDefaultForType }?.let { "Min $it" },
        fact.maxString.takeIf { it.isNotBlank() && !fact.maxIsDefaultForType }?.let { "Max $it" },
        fact.defaultValueString.takeIf { it.isNotBlank() }?.let { "Default $it" },
    )
    return parts.takeIf { it.isNotEmpty() }?.joinToString(" · ")
}

internal fun factRebootNote(fact: Fact): String? = when {
    fact.vehicleRebootRequired -> "Reboot the vehicle for this to take effect."
    fact.qgcRebootRequired -> "Restart Aircast for this to take effect."
    else -> null
}

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
            modifier = Modifier.fillMaxWidth(),
            trailingIcon = {
                val committed = editing
                if (committed != null && committed != fact.valueString) {
                    TextButton(onClick = {
                        scope.launch {
                            val refused = rejectionFor(fact, committed)
                            rejection = refused
                            if (refused == null) {
                                editing = null
                                withContext(Dispatchers.Default) { Qgc.set(fact.path, committed) }
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
