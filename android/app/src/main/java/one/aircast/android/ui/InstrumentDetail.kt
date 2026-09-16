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

internal fun gpsDetail(count: String?, fix: FixLevel?, hdop: String?, vdop: String?, course: String?): List<DetailRow> =
    listOfNotNull(
        lockName(fix)?.let { DetailRow("GPS lock", it) },
        count?.takeIf { it.isNotBlank() }?.let { DetailRow("Satellites", it) },
        hdop?.takeIf { usableDop(it) }?.let { DetailRow("HDOP", it) },
        vdop?.takeIf { usableDop(it) }?.let { DetailRow("VDOP", it) },
        course?.takeIf { !notYetComputed(it) }?.let { DetailRow("Course over ground", "$it°") },
    )

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
