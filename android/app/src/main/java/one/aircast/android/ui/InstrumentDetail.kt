package one.aircast.android.ui

import one.aircast.map.optText
import org.json.JSONObject

internal const val SEVERITY_SECONDARY = -1

internal data class DetailRow(val label: String, val value: String, val severity: Int = 0)

internal data class BatteryHeadline(
    val text: String,
    val detail: String,
    val severity: Int,
    val level: BatteryLevel = BatteryLevel.Normal,
    val margin: String = "",
    val index: Int = 0,
)

internal fun batteryHeadline(view: JSONObject?): BatteryHeadline? =
    view?.takeIf { it.optBoolean("available") }?.optJSONObject("headline")?.let {
        BatteryHeadline(it.optText("text"), it.optText("detail"), it.optInt("severity"), batteryLevelOf(it.optText("level")), it.optText("margin"), it.optInt("index"))
    }

private const val LIMITING_PACK = "lowest"

internal fun batteryDetail(view: JSONObject?): List<DetailRow> {
    val packs = view?.takeIf { it.optBoolean("available") }?.optJSONArray("packs") ?: return emptyList()
    val rowsOf = { index: Int ->
        packs.optJSONObject(index)?.optJSONArray("rows")?.let { rows -> (0 until rows.length()).mapNotNull { rows.optJSONObject(it) } }.orEmpty()
    }
    if (packs.length() < 2) return rowsOf(0).map { DetailRow(it.optText("label"), it.optText("value"), it.optInt("severity", SEVERITY_SECONDARY)) }
    val limiting = batteryHeadline(view)?.index
    return (0 until packs.length()).map { index ->
        val rows = rowsOf(index)
        DetailRow(
            listOfNotNull("Battery ${index + 1}", LIMITING_PACK.takeIf { index == limiting }).joinToString(" \u00b7 "),
            rows.joinToString(" \u00b7 ") { it.optText("value") },
            rows.maxOfOrNull { it.optInt("severity", SEVERITY_SECONDARY) }?.coerceAtLeast(0) ?: 0,
        )
    }
}

private const val SECONDS_PER_MINUTE = 60

internal fun silenceDuration(seconds: Long): String =
    if (seconds < SECONDS_PER_MINUTE) "$seconds s" else "${seconds / SECONDS_PER_MINUTE} min"

internal fun silenceText(seconds: Long): String = "No data from the aircraft for ${silenceDuration(seconds)}"

internal const val SIGNAL_LOST = "Signal lost"

internal fun signalLostTitle(seconds: Long?): String =
    listOfNotNull(SIGNAL_LOST, seconds?.let(::silenceDuration)).joinToString(" \u00b7 ")

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
    "timeRemainingStr" -> "Time remaining"
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
