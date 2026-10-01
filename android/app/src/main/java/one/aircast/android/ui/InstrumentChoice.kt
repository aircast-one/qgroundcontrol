package one.aircast.android.ui

import android.content.Context
import org.json.JSONObject
import one.aircast.mapspike.optText

internal const val INSTRUMENT_GROUPS = "view.instrumentGroups"

internal val DEFAULT_INSTRUMENTS = listOf(
    "distanceToHome",
    "altitudeRelative",
    "groundSpeed",
    "climbRate",
)

private const val INSTRUMENTS_VIEW = "view.instruments"

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

internal fun instrumentsPath(chosen: List<String>): String = when {
    chosen.isEmpty() || chosen == DEFAULT_INSTRUMENTS -> INSTRUMENTS_VIEW
    else -> "$INSTRUMENTS_VIEW(${chosen.joinToString(",")})"
}

internal fun showsInstruments(chosen: List<String>): Boolean = chosen.isNotEmpty()

internal fun withInstrument(chosen: List<String>, name: String): List<String> = when {
    name in chosen -> chosen - name
    chosen.size >= MOST_INSTRUMENTS -> chosen
    else -> chosen + name
}

internal fun emptyCatalogueText(connected: Boolean): String = when {
    connected -> "This vehicle reported no readings this screen can ask for."
    else -> "Connect a vehicle to see what it can report."
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
