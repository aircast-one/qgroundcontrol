package one.aircast.mapspike

import org.json.JSONObject

// A plan can hold complex items this map cannot draw — corridor and structure
// scans, landing patterns — and a plan opened from a file is where they arrive.
// Drawing part of a plan without saying so is the dangerous half: the items are
// still there, still counted in the distance, and still uploaded to the aircraft.
fun undrawnComplexItems(json: JSONObject?, surveys: List<Survey>): List<String> {
    val elements = json?.optJSONArray("elements") ?: return emptyList()
    val drawnSurveys = surveys.map { it.index }.toSet()

    return (0 until elements.length()).mapNotNull { index ->
        val element = elements.optJSONObject(index) ?: return@mapNotNull null
        if (element.optBoolean("isSimpleItem", true)) {
            return@mapNotNull null
        }
        if (index in drawnSurveys) {
            return@mapNotNull null
        }
        element.optString("commandName").takeIf { it.isNotBlank() }
    }.distinct()
}

fun undrawnLabel(names: List<String>): String = when {
    names.isEmpty() -> ""
    else -> " · ${names.joinToString(", ")} not drawn"
}
