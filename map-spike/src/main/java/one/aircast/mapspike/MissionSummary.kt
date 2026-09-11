package one.aircast.mapspike

import org.json.JSONObject

internal fun summaryRow(view: JSONObject?, label: String): String? {
    val rows = view?.optJSONArray("rows") ?: return null
    return (0 until rows.length())
        .mapNotNull { rows.optJSONObject(it) }
        .firstOrNull { it.optString("label") == label }
        ?.optString("value")
        ?.takeIf { it.isNotBlank() }
}

// The core withholds the row at zero; one is withheld here because a mission
// that takes one battery is the ordinary case and saying so is noise. Two or
// more is a fact an operator wants before leaving for the field.
internal fun batteriesText(view: JSONObject?): String? =
    summaryRow(view, "Batteries")
        ?.toIntOrNull()
        ?.takeIf { it > 1 }
        ?.let { "$it batteries" }

fun missionSummaryText(view: JSONObject?): String =
    listOfNotNull(
        summaryRow(view, "Distance"),
        summaryRow(view, "Time"),
        batteriesText(view),
    ).joinToString(" · ")
