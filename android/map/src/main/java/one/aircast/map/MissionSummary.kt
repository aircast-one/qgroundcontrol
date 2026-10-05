package one.aircast.map

import org.json.JSONObject

fun summaryRow(view: JSONObject?, label: String): String? {
    val rows = view?.optJSONArray("rows") ?: return null
    return (0 until rows.length())
        .mapNotNull { rows.optJSONObject(it) }
        .firstOrNull { it.optText("label") == label }
        ?.optText("value")
        ?.takeIf { it.isNotBlank() }
}

fun missionSummaryText(view: JSONObject?): String =
    listOfNotNull(
        summaryRow(view, "Distance")?.let { "Distance $it" },
        summaryRow(view, "Time")?.let { "Time $it" },
        summaryRow(view, "Furthest from launch")?.let { "Max telem $it" },
    ).joinToString(" · ")
