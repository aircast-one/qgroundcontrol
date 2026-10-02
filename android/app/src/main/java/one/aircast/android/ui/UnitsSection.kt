package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
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
    return facts.filterNot { it.name == "customUnits" || it.name == "weightUnits" }
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
    ChoiceField("Measurement system", unitSystemLabel(system), UNIT_SYSTEM_LABELS, Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp), onPick = onPick)
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
        FootNote(unitSystemNote(chosen))
        FactRuns(unitRowsFor(chosen, facts))
    }
}
