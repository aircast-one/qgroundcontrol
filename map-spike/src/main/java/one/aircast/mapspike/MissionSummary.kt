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

fun missionSummaryText(view: JSONObject?): String =
    listOfNotNull(summaryRow(view, "Distance"), summaryRow(view, "Time")).joinToString(" · ")
