package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
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
import one.aircast.android.bridge.settingControl
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcDouble
import one.aircast.android.bridge.qgcPath
import one.aircast.mapspike.optText
import org.json.JSONObject

private const val UNITS_PATH = "settings.unitsSettings"

internal const val UNIT_SYSTEM_CUSTOM = 2

internal val UNIT_SYSTEM_LABELS = listOf("Metric", "Imperial", "Custom")

internal val PRESET_UNIT_FACTS = listOf(
    "horizontalDistanceUnits",
    "verticalDistanceUnits",
    "areaUnits",
    "speedUnits",
    "temperatureUnits",
)

private const val GENERAL_SETTINGS = "view.settings(General)"

// view.settings(General) carries the units group already decoded - visibility applied, each fact a
// control - so the rows come from there rather than from the raw settings.unitsSettings group.
internal fun unitFacts(page: JSONObject?): List<Fact> {
    val sections = page?.optJSONArray("sections") ?: return emptyList()
    val units = (0 until sections.length()).mapNotNull { sections.optJSONObject(it) }
        .firstOrNull { it.optText("group") == "unitsSettings" } ?: return emptyList()
    val subsections = units.optJSONArray("subsections") ?: return emptyList()
    return (0 until subsections.length()).flatMap { index ->
        val controls = subsections.optJSONObject(index)?.optJSONArray("controls")
        (0 until (controls?.length() ?: 0)).mapNotNull { controls!!.optJSONObject(it)?.let(::factFromControl) }
    }
}

internal fun unitRowsFor(system: Int, facts: List<Fact>): List<Fact> {
    val custom = unitSystemLabel(system) == UNIT_SYSTEM_LABELS[UNIT_SYSTEM_CUSTOM]
    return facts.filterNot { it.name == "customUnits" }
        .filterNot { !custom && it.name in PRESET_UNIT_FACTS }
}

internal fun unitSystemLabel(system: Int): String =
    UNIT_SYSTEM_LABELS.getOrElse(system) { UNIT_SYSTEM_LABELS[UNIT_SYSTEM_CUSTOM] }

internal fun unitSystemNote(system: Int): String =
    if (unitSystemLabel(system) == UNIT_SYSTEM_LABELS[UNIT_SYSTEM_CUSTOM]) {
        "Each measurement is set on its own below."
    } else {
        "Distance, area, speed and temperature all follow ${unitSystemLabel(system)}. " +
            "Choose Custom to set them one at a time."
    }

@Composable
private fun UnitSystemRow(system: Int, onPick: (Int) -> Unit) {
    var open by remember { mutableStateOf(false) }

    Row(
        Modifier
            .fillMaxWidth()
            .clickable { open = true }
            .heightIn(min = 64.dp)
            .padding(horizontal = 20.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text(
            text = "Measurement system",
            style = MaterialTheme.typography.titleMedium,
            modifier = Modifier.weight(1f),
        )
        Box {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    text = unitSystemLabel(system),
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.primary,
                )
                Icon(
                    Icons.Filled.KeyboardArrowDown,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                )
            }
            DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
                UNIT_SYSTEM_LABELS.forEachIndexed { index, label ->
                    DropdownMenuItem(
                        text = { Text(label) },
                        onClick = {
                            open = false
                            onPick(index)
                        },
                    )
                }
            }
        }
    }
}

@Composable
fun UnitsSection(modifier: Modifier = Modifier) {
    val page by qgcPath(GENERAL_SETTINGS)
    val facts = remember(page) { unitFacts(page) }
    val system by qgcDouble(settingControl("$UNITS_PATH.unitSystem"), 0.0)
    val chosen = system.toInt()

    Column(modifier) {
        UnitSystemRow(chosen) { picked ->
            offMainDetached { Qgc.invoke("$UNITS_PATH.setUnitSystem", picked) }
        }
        HorizontalDivider()
        FootNote(unitSystemNote(chosen))
        unitRowsFor(chosen, facts).forEach { fact ->
            FactRow(fact)
            HorizontalDivider()
        }
    }
}
