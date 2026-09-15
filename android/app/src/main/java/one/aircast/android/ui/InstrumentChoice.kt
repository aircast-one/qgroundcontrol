package one.aircast.android.ui

import android.content.Context
import org.json.JSONObject
import one.aircast.mapspike.optText

internal const val INSTRUMENT_GROUPS = "view.instrumentGroups"

internal const val VEHICLE_FACTS = "vehicle"

internal val DEFAULT_INSTRUMENTS = listOf(
    "altitudeRelative",
    "groundSpeed",
    "distanceToHome",
    "heading",
)

internal const val MOST_INSTRUMENTS = 6

internal data class InstrumentFact(val name: String, val label: String, val path: String)

internal data class InstrumentGroup(val group: String, val title: String, val facts: List<InstrumentFact>)

internal fun instrumentGroups(view: JSONObject?): List<InstrumentGroup> {
    val listed = view?.takeIf { it.optBoolean("available") }?.optJSONArray("groups") ?: return emptyList()
    return (0 until listed.length()).mapNotNull { index ->
        listed.optJSONObject(index)?.let { entry ->
            val facts = entry.optJSONArray("facts")
            InstrumentGroup(
                group = entry.optText("group"),
                title = entry.optText("title"),
                facts = (0 until (facts?.length() ?: 0)).mapNotNull { fact ->
                    facts!!.optJSONObject(fact)?.let {
                        val name = it.optText("name")
                        InstrumentFact(
                            name = name,
                            label = it.optText("label"),
                            path = "${entry.optText("group")}.$name",
                        )
                    }
                }.filter { it.name.isNotBlank() },
            )
        }
    }.filter { it.facts.isNotEmpty() }
}

internal fun vehicleOwnGroup(view: JSONObject?): InstrumentGroup? {
    val facts = view?.takeIf { it.optText("kind") == "object" }?.optJSONArray("facts") ?: return null
    val listed = (0 until facts.length()).mapNotNull { index ->
        facts.optJSONObject(index)?.let { fact ->
            val property = fact.optText("property")
            InstrumentFact(
                name = property,
                label = fact.optText("shortDescription").ifBlank { property },
                path = property,
            )
        }
    }.filter { it.name.isNotBlank() }
    return listed.takeIf { it.isNotEmpty() }?.let { InstrumentGroup("vehicle", "Vehicle", it) }
}

internal fun instrumentsPath(chosen: List<String>): String =
    "view.instruments(${chosen.ifEmpty { DEFAULT_INSTRUMENTS }.joinToString(",")})"

internal fun showsInstruments(chosen: List<String>): Boolean = chosen.isNotEmpty()

internal fun withInstrument(chosen: List<String>, name: String): List<String> = when {
    name in chosen -> chosen - name
    chosen.size >= MOST_INSTRUMENTS -> chosen
    else -> chosen + name
}

internal fun instrumentChoiceNote(chosen: List<String>): String = when (chosen.size) {
    0 -> "Nothing chosen. The flight screen shows no readings."
    MOST_INSTRUMENTS -> "$MOST_INSTRUMENTS is as many as the row fits. Remove one to add another."
    else -> "${chosen.size} of $MOST_INSTRUMENTS chosen."
}

private const val STORE = "fly-instruments"
private const val KEY = "chosen"

internal fun readChosen(context: Context): List<String> =
    context.getSharedPreferences(STORE, Context.MODE_PRIVATE)
        .getString(KEY, null)
        ?.split(",")
        ?.filter { it.isNotBlank() }
        ?: DEFAULT_INSTRUMENTS

internal fun writeChosen(context: Context, chosen: List<String>) {
    context.getSharedPreferences(STORE, Context.MODE_PRIVATE)
        .edit()
        .putString(KEY, chosen.joinToString(","))
        .apply()
}
