package one.aircast.mapspike

import org.json.JSONObject

// A plan can hold complex items this map cannot draw — corridor and structure
// scans, landing patterns — and a plan opened from a file is where they arrive.
// Drawing part of a plan without saying so is the dangerous half: the items are
// still there, still counted in the distance, and still uploaded to the aircraft.
fun undrawnComplexItems(
    json: JSONObject?,
    items: List<MissionItem>,
    surveys: List<Survey>,
): List<String> {
    val elements = json?.optJSONArray("elements") ?: return emptyList()
    // Complex is not the same as undrawn. Mission Start is a complex item and is
    // drawn as a numbered marker, so the question is whether anything on the map
    // came from this element, not what kind of element it is.
    val drawn = items.map { it.index }.toSet() + surveys.map { it.index }.toSet()

    return (0 until elements.length()).mapNotNull { index ->
        val element = elements.optJSONObject(index) ?: return@mapNotNull null
        if (element.optBoolean("isSimpleItem", true) || index in drawn) {
            return@mapNotNull null
        }
        element.optString("commandName").takeIf { it.isNotBlank() }
    }.distinct()
}

fun undrawnLabel(names: List<String>): String = when {
    names.isEmpty() -> ""
    else -> " · ${names.joinToString(", ")} not drawn"
}
