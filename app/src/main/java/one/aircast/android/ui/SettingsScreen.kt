package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
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
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcFacts

data class SettingsGroup(val path: String, val title: String)

const val LINKS_GROUP_PATH = "links"

val SETTINGS_GROUPS = listOf(
    SettingsGroup(LINKS_GROUP_PATH, "Comm Links"),
    SettingsGroup("settings.unitsSettings", "Units"),
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
                ListItem(
                    headlineContent = { Text(entry.title) },
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
        items(facts, key = { it.path }) { fact ->
            FactRow(fact)
            HorizontalDivider()
        }
    }
}

@Composable
private fun FactRow(fact: Fact) {
    Row(
        Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Column(Modifier.weight(1f)) {
            Text(fact.title, style = MaterialTheme.typography.bodyLarge)
            if (fact.units.isNotBlank()) {
                Text(fact.units, style = MaterialTheme.typography.bodySmall)
            }
        }

        when {
            fact.readOnly -> Text(fact.valueString, style = MaterialTheme.typography.bodyMedium)
            fact.isBool -> Switch(
                checked = fact.boolValue,
                onCheckedChange = { Qgc.set(fact.path, it) },
            )
            fact.isEnum -> EnumPicker(fact)
            else -> FactTextField(fact)
        }
    }
}

@Composable
private fun EnumPicker(fact: Fact) {
    var expanded by remember { mutableStateOf(false) }
    val label = fact.enumStrings.getOrNull(fact.enumIndex) ?: fact.valueString

    Column {
        TextButton(onClick = { expanded = true }) {
            Text(label)
            Icon(Icons.Default.KeyboardArrowDown, null)
        }
        DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            fact.enumStrings.forEachIndexed { index, option ->
                DropdownMenuItem(
                    text = { Text(option) },
                    onClick = {
                        expanded = false
                        Qgc.set("${fact.path}.enumIndex", index)
                    },
                )
            }
        }
    }
}

@Composable
private fun FactTextField(fact: Fact) {
    var editing by remember(fact.path) { mutableStateOf<String?>(null) }

    OutlinedTextField(
        value = editing ?: fact.valueString,
        onValueChange = { editing = it },
        singleLine = true,
        modifier = Modifier.width(160.dp),
        trailingIcon = {
            if (editing != null && editing != fact.valueString) {
                TextButton(onClick = {
                    Qgc.set(fact.path, editing)
                    editing = null
                }) { Text("Set") }
            }
        },
    )
}
