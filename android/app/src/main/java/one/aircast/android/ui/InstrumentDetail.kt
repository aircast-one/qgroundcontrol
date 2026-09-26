package one.aircast.android.ui

import one.aircast.mapspike.optText
import org.json.JSONObject

internal data class DetailRow(val label: String, val value: String)

internal fun batteryDetail(view: JSONObject?): List<DetailRow> {
    val packs = view?.takeIf { it.optBoolean("available") }?.optJSONArray("packs") ?: return emptyList()
    val many = packs.length() > 1
    return (0 until packs.length()).flatMap { index ->
        val pack = packs.optJSONObject(index) ?: return@flatMap emptyList()
        val prefix = if (many) "Battery ${index + 1} " else ""
        val facts = pack.optJSONArray("facts")
        val charge = pack.optText("chargeLabel").takeIf { it.isNotBlank() && it != "n/a" }
        (0 until (facts?.length() ?: 0)).mapNotNull { at ->
            facts?.optJSONObject(at)?.let { fact ->
                val value = fact.optText("valueString")
                if (notYetComputed(value)) return@let null
                DetailRow(
                    label = prefix + factLabel(fact.optText("name")),
                    value = listOf(value, fact.optText("units")).filter { it.isNotBlank() }.joinToString(" "),
                )
            }
        } + listOfNotNull(charge?.let { DetailRow("${prefix}State", it) })
    }
}

internal fun factLabel(name: String): String = when (name) {
    "voltage" -> "Voltage"
    "current" -> "Current"
    "instantPower" -> "Power"
    "mahConsumed" -> "Consumed"
    "timeRemainingStr" -> "Time remaining"
    "temperature" -> "Temperature"
    "percentRemaining" -> "Remaining"
    else -> name.replaceFirstChar { it.uppercase() }
}

internal fun lockName(fix: FixLevel?): String? = when (fix) {
    null -> null
    FixLevel.None -> "No fix"
    FixLevel.TwoD -> "2D"
    FixLevel.Good -> "3D or better"
}

internal data class GpsStatus(val satellites: Int?, val lock: Double, val rows: List<DetailRow>)

// view.gps serves the satellites, the lock and the detail rows - which facts, in which order, and
// which are not yet reported, including a dilution the receiver sends as 655.35 for "unknown".
internal fun gpsStatus(view: JSONObject?): GpsStatus? {
    if (view == null || !view.optBoolean("available")) return null
    val rows = view.optJSONArray("rows")
    return GpsStatus(
        satellites = if (view.isNull("satellites")) null else view.optInt("satellites"),
        lock = if (view.isNull("lock")) Double.NaN else view.optDouble("lock", Double.NaN),
        rows = (0 until (rows?.length() ?: 0)).mapNotNull { index ->
            rows?.optJSONObject(index)?.let { DetailRow(it.optString("label"), it.optString("value")) }
        }.filter { it.label.isNotBlank() && it.value.isNotBlank() },
    )
}

internal fun gpsDetail(fix: FixLevel?, gps: GpsStatus?): List<DetailRow> =
    listOfNotNull(lockName(fix)?.let { DetailRow("GPS lock", it) }) + (gps?.rows ?: emptyList())

internal fun notYetComputed(shown: String): Boolean =
    shown.isBlank() || shown.all { it == '-' || it == ':' || it == '.' || it == ' ' }

internal fun usableDop(shown: String): Boolean =
    !notYetComputed(shown) && shown.toDoubleOrNull()?.let { it > 0.0 && it < 100.0 } == true

internal fun linkDetail(links: VehicleLinks?, names: List<String>, primary: String?): List<DetailRow> {
    if (links == null || !links.available) return emptyList()
    return names.mapIndexed { index, name ->
        val lost = links.links.getOrNull(index)?.commLost == true
        DetailRow(
            label = name,
            value = listOfNotNull(
                if (name == primary) "carrying" else null,
                if (lost) "no contact" else null,
            ).ifEmpty { listOf("standing by") }.joinToString(" · "),
        )
    }
}
