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

private val CRITICAL_CHARGE_STATES = 3..6

internal fun batteryReturnOffered(view: JSONObject?): Boolean {
    val packs = view?.takeIf { it.optBoolean("available") }?.optJSONArray("packs") ?: return false
    return (0 until packs.length()).any { packs.optJSONObject(it)?.optInt("chargeState", 0) in CRITICAL_CHARGE_STATES }
}

internal fun totalDraw(view: JSONObject?): String? {
    val packs = view?.takeIf { it.optBoolean("available") }?.optJSONArray("packs") ?: return null
    val number = { pack: JSONObject, name: String ->
        pack.optJSONArray("facts")?.let { facts -> (0 until facts.length()).mapNotNull { facts.optJSONObject(it) }.firstOrNull { it.optText("name") == name } }
            ?.optDouble("value")?.takeIf { !it.isNaN() }
    }
    val all = (0 until packs.length()).mapNotNull { packs.optJSONObject(it) }
    val watts = all.map { number(it, "instantPower") }
    val amps = all.map { number(it, "current") }
    return when {
        all.isEmpty() -> null
        watts.all { it != null } -> "${Math.round(watts.sumOf { it ?: 0.0 })}W"
        amps.all { it != null } -> "%.1fA".format(java.util.Locale.ROOT, amps.sumOf { it ?: 0.0 })
        else -> null
    }
}

internal fun factLabel(name: String): String = when (name) {
    "voltage" -> "Voltage"
    "current" -> "Current"
    "instantPower" -> "Power"
    "mahConsumed" -> "Consumed"
    "timeRemainingStr" -> "Time Remaining"
    "temperature" -> "Temperature"
    "percentRemaining" -> "Remaining"
    else -> name.replaceFirstChar { it.uppercase() }
}

internal data class GpsStatus(val satellites: Int?, val lock: Double, val lockText: String, val rows: List<DetailRow>)

internal fun gpsStatus(view: JSONObject?): GpsStatus? {
    if (view == null || !view.optBoolean("available")) return null
    val rows = view.optJSONArray("rows")
    return GpsStatus(
        satellites = if (view.isNull("satellites")) null else view.optInt("satellites"),
        lock = if (view.isNull("lock")) Double.NaN else view.optDouble("lock", Double.NaN),
        lockText = view.optText("lockText"),
        rows = (0 until (rows?.length() ?: 0)).mapNotNull { index ->
            rows?.optJSONObject(index)?.let { DetailRow(it.optString("label"), it.optString("value")) }
        }.filter { it.label.isNotBlank() && it.value.isNotBlank() },
    )
}

internal fun gpsDetail(gps: GpsStatus?): List<DetailRow> =
    listOfNotNull(gps?.lockText?.takeIf { it.isNotBlank() }?.let { DetailRow("GPS Lock", it) }) + (gps?.rows ?: emptyList())

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
