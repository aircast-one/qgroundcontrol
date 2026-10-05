package one.aircast.android.ui

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import org.json.JSONObject
import one.aircast.map.optText

internal const val INSTRUMENT_GROUPS = "view.instrumentGroups"

internal val DEFAULT_INSTRUMENTS = listOf(
    "distanceToHome",
    "altitudeRelative",
    "groundSpeed",
    "climbRate",
)

private val FORWARD_FLIGHT_CLASSES = setOf("fixedWing", "vtol", "airship")

internal fun defaultInstruments(vehicleClass: String): List<String> =
    DEFAULT_INSTRUMENTS + listOf("airSpeed").filter { vehicleClass in FORWARD_FLIGHT_CLASSES }

internal const val INSTRUMENTS_VIEW = "view.instruments"

internal data class InstrumentFact(val name: String, val label: String, val path: String)

internal data class InstrumentGroup(val group: String, val title: String, val facts: List<InstrumentFact>)

internal fun instrumentGroups(view: JSONObject?): List<InstrumentGroup> =
    listedGroups(view, "groups") + listedGroups(view, "packGroups")

private fun listedGroups(view: JSONObject?, key: String): List<InstrumentGroup> {
    val listed = view?.takeIf { it.optBoolean("available") }?.optJSONArray(key) ?: return emptyList()
    return (0 until listed.length()).mapNotNull { index ->
        listed.optJSONObject(index)?.let { entry ->
            val facts = entry.optJSONArray("facts")
            InstrumentGroup(
                group = entry.optText("group"),
                title = entry.optText("title"),
                facts = (0 until (facts?.length() ?: 0)).mapNotNull { fact ->
                    facts!!.optJSONObject(fact)?.let {
                        InstrumentFact(
                            name = it.optText("name"),
                            label = it.optText("label"),
                            path = it.optText("selection"),
                        )
                    }
                }.filter { it.name.isNotBlank() && it.path.isNotBlank() },
            )
        }
    }.filter { it.facts.isNotEmpty() }
}

internal fun vehicleOwnGroup(view: JSONObject?): InstrumentGroup? {
    val facts = view?.takeIf { it.optBoolean("available") }?.optJSONArray("vehicleFacts") ?: return null
    val listed = (0 until facts.length()).mapNotNull { index ->
        facts.optJSONObject(index)?.let { fact ->
            InstrumentFact(
                name = fact.optText("name"),
                label = fact.optText("label"),
                path = fact.optText("selection"),
            )
        }
    }.filter { it.name.isNotBlank() && it.path.isNotBlank() }
    return listed.takeIf { it.isNotEmpty() }?.let { InstrumentGroup("vehicle", "Vehicle", it) }
}

internal fun instrumentsPath(chosen: List<String>, vehicleClass: String = GENERIC_CLASS): String = when {
    chosen.isEmpty() || chosen == defaultInstruments(vehicleClass) -> INSTRUMENTS_VIEW
    else -> "$INSTRUMENTS_VIEW(${chosen.joinToString(",")})"
}

internal fun showsInstruments(chosen: List<String>): Boolean = chosen.isNotEmpty()

internal fun withInstrument(chosen: List<String>, name: String): List<String> =
    if (name in chosen) chosen - name else chosen + name

internal fun selectionId(selection: String): String = if ('/' in selection) selection else "vehicle/$selection"

internal fun movedInstrument(chosen: List<String>, index: Int, by: Int): List<String> =
    (index + by).takeIf { it in chosen.indices && index in chosen.indices }
        ?.let { to -> chosen.mapIndexed { i, name -> when (i) { index -> chosen[to]; to -> chosen[index]; else -> name } } }
        ?: chosen

internal fun replacedInstrument(chosen: List<String>, index: Int, path: String): List<String> =
    chosen.mapIndexed { i, name -> if (i == index) path else name }.filterIndexed { i, name -> i == index || name != path }

internal fun removedInstrument(chosen: List<String>, index: Int): List<String> =
    chosen.filterIndexed { i, _ -> i != index }

internal fun emptyCatalogueText(connected: Boolean): String = when {
    connected -> "This vehicle reported no readings this screen can ask for."
    else -> "Connect a vehicle to see what it can report."
}

internal fun instrumentChoiceNote(chosen: List<String>): String = when (chosen.size) {
    0 -> "Nothing chosen. The flight screen shows no readings."
    else -> "${chosen.size} chosen."
}

private const val STORE = "fly-instruments"
private const val GENERIC_CLASS = "generic"

internal fun instrumentVehicleClass(view: JSONObject?): String =
    view?.optText("vehicleClass")?.ifBlank { null } ?: GENERIC_CLASS

internal fun chosenKey(vehicleClass: String): String = "chosen-$vehicleClass"

internal fun readChosen(context: Context, vehicleClass: String): List<String> =
    context.getSharedPreferences(STORE, Context.MODE_PRIVATE)
        .getString(chosenKey(vehicleClass), null)
        ?.split(",")
        ?.filter { it.isNotBlank() }
        ?: defaultInstruments(vehicleClass)

internal object InstrumentEdits {
    var version by androidx.compose.runtime.mutableIntStateOf(0)
}

internal fun writeChosen(context: Context, vehicleClass: String, chosen: List<String>) {
    InstrumentEdits.version += 1
    context.getSharedPreferences(STORE, Context.MODE_PRIVATE)
        .edit()
        .putString(chosenKey(vehicleClass), chosen.joinToString(","))
        .apply()
    offMainDetached { Qgc.invoke("subtitles.setInstruments", vehicleClass, chosen.joinToString(",")) }
}

private const val VALUE_SIZE_KEY = "fontSize"

internal enum class ValueSize(val label: String, val scale: Float) {
    Default("Default", 1f),
    Small("Small", 0.86f),
    Medium("Medium", 1.25f),
    Large("Large", 1.5f),
}

internal fun valueSizeAt(ordinal: Int): ValueSize = ValueSize.entries.getOrElse(ordinal) { ValueSize.Default }

internal fun nextValueSize(size: ValueSize): ValueSize = valueSizeAt((size.ordinal + 1) % ValueSize.entries.size)

internal fun valueSizePillText(size: ValueSize): String = "Size: ${size.label}"

internal fun readValueSize(context: Context, vehicleClass: String): ValueSize =
    valueSizeAt(context.getSharedPreferences(STORE, Context.MODE_PRIVATE).getInt("$VALUE_SIZE_KEY-$vehicleClass", 0))

internal fun writeValueSize(context: Context, vehicleClass: String, size: ValueSize) {
    context.getSharedPreferences(STORE, Context.MODE_PRIVATE).edit().putInt("$VALUE_SIZE_KEY-$vehicleClass", size.ordinal).apply()
}

internal fun shownFirst(groups: List<InstrumentGroup>, shown: List<String>): List<InstrumentGroup> {
    val all = groups.flatMap { it.facts }
    val picked = InstrumentGroup(SHOWN_GROUP, "Shown", shown.mapNotNull { path -> all.firstOrNull { it.path == path } })
    return (listOf(picked) + groups.map { group -> group.copy(facts = group.facts.filterNot { it.path in shown }) })
        .filter { it.facts.isNotEmpty() }
}

private const val SHOWN_GROUP = "shown"
