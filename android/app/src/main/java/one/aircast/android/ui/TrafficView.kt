package one.aircast.android.ui

import one.aircast.mapspike.optText
import org.json.JSONObject

internal const val TRAFFIC_VIEW = "view.adsbTraffic"

internal enum class TrafficLevel { Good, Caution, Warning, Critical }

internal data class TrafficContact(
    val icaoAddress: Int,
    val callsign: String,
    val distance: Double?,
    val bearingDegrees: Double?,
    val altitude: Double?,
    val relativeAltitude: Double?,
    val emergency: String,
    val alert: Boolean?,
    val stale: Boolean,
    val altitudeType: String = "",
) {
    val name: String get() = callsign.ifBlank { "%06X".format(icaoAddress) }
    val located: Boolean get() = distance != null && bearingDegrees != null
}

internal data class TrafficUnits(val distance: String, val altitude: String, val heading: String)

internal data class TrafficReading(
    val enabled: Boolean,
    val available: Boolean,
    val connected: Boolean,
    val receiving: Boolean,
    val ownPositionKnown: Boolean,
    val alerting: Boolean?,
    val alertUnknown: Int,
    val emergency: String,
    val errorToken: String,
    val units: TrafficUnits,
    val contacts: List<TrafficContact>,
)

private fun JSONObject.number(name: String): Double? = optDouble(name).takeIf { it.isFinite() }

private fun JSONObject.flagOrNull(name: String): Boolean? = if (isNull(name)) null else optBoolean(name)

internal fun trafficReading(view: JSONObject?): TrafficReading? {
    if (view == null || view.optText("class") != "AdsbTraffic") return null
    val contacts = view.optJSONArray("contacts")
    val units = view.optJSONObject("units") ?: JSONObject()
    return TrafficReading(
        enabled = view.optBoolean("enabled"),
        available = view.optBoolean("available"),
        connected = view.optBoolean("connected"),
        receiving = view.optBoolean("receiving"),
        ownPositionKnown = view.optBoolean("ownPositionKnown"),
        alerting = view.flagOrNull("alerting"),
        alertUnknown = view.optInt("alertUnknown"),
        emergency = view.optText("emergency"),
        errorToken = view.optJSONObject("error")?.optText("token").orEmpty(),
        units = TrafficUnits(
            distance = units.optText("distance"),
            altitude = units.optText("altitude"),
            heading = units.optText("heading"),
        ),
        contacts = (0 until (contacts?.length() ?: 0)).mapNotNull { index ->
            contacts?.optJSONObject(index)?.let { contact ->
                TrafficContact(
                    icaoAddress = contact.optInt("icaoAddress"),
                    callsign = contact.optText("callsign").trim(),
                    distance = contact.number("distance"),
                    bearingDegrees = contact.number("bearingDegrees"),
                    altitude = contact.number("altitude"),
                    relativeAltitude = contact.number("relativeAltitude"),
                    emergency = contact.optText("emergency"),
                    alert = contact.flagOrNull("alert"),
                    stale = contact.optBoolean("stale"),
                    altitudeType = contact.optText("altitudeType"),
                )
            }
        },
    )
}

internal fun trafficShown(reading: TrafficReading): Boolean =
    reading.enabled || reading.contacts.isNotEmpty()

internal fun trafficLevel(reading: TrafficReading): TrafficLevel = when {
    reading.emergency.isNotBlank() -> TrafficLevel.Critical
    reading.alerting == true -> TrafficLevel.Warning
    reading.errorToken.isNotBlank() -> TrafficLevel.Caution
    !reading.receiving -> TrafficLevel.Caution
    reading.alertUnknown > 0 && reading.connected -> TrafficLevel.Caution
    else -> TrafficLevel.Good
}

internal fun trafficSummary(reading: TrafficReading): String = when {
    reading.contacts.size == 1 -> "Traffic: 1 aircraft"
    reading.contacts.isNotEmpty() -> "Traffic: ${reading.contacts.size} aircraft"
    reading.errorToken == "connectFailed" -> "Traffic server unreachable"
    reading.errorToken == "linkLost" -> "Traffic feed dropped"
    reading.errorToken.isNotBlank() -> "Traffic feed failed"
    !reading.available -> "No traffic receiver"
    !reading.receiving -> "No traffic feed"
    else -> "Traffic clear"
}

internal fun trafficContactUrgent(contact: TrafficContact): Boolean =
    contact.alert == true || contact.emergency.isNotBlank()

internal fun trafficCaption(reading: TrafficReading): String = when (reading.ownPositionKnown) {
    false -> "No vehicle position, so nothing can be ranged"
    true -> "Range, bearing and height are relative to the vehicle"
}

internal fun trafficEmergencyText(token: String): String = when (token) {
    "hijack" -> "squawking hijack"
    "radioFailure" -> "squawking radio failure"
    "general" -> "squawking emergency"
    else -> ""
}

private fun reading(value: Double?, unit: String, fine: Boolean = false): String {
    val number = value ?: return ""
    val printed = if (fine && kotlin.math.abs(number) < 10.0) "%.1f".format(number) else "%.0f".format(number)
    return if (unit.isBlank()) printed else "$printed $unit"
}

internal fun trafficDatumText(altitudeType: String): String = when (altitudeType) {
    "pressureQnh" -> "by pressure"
    "geometric" -> "by GPS"
    else -> ""
}

internal fun trafficHeightText(contact: TrafficContact, units: TrafficUnits): String {
    val relative = contact.relativeAltitude ?: return listOf(
        reading(contact.altitude, units.altitude),
        trafficDatumText(contact.altitudeType),
    ).filter { it.isNotBlank() }.joinToString(" ")
    val printed = reading(kotlin.math.abs(relative), units.altitude)
    return when {
        kotlin.math.abs(relative) < 0.5 -> "my level"
        relative > 0 -> "$printed above"
        else -> "$printed below"
    }
}

internal fun trafficContactText(contact: TrafficContact, units: TrafficUnits): String {
    val position = when {
        contact.located -> listOf(
            reading(contact.distance, units.distance, fine = true),
            reading(contact.bearingDegrees, units.heading),
            trafficHeightText(contact, units),
        )
        else -> listOf("bearing unknown", trafficHeightText(contact, units))
    }
    val trailing = listOf(
        trafficEmergencyText(contact.emergency),
        if (contact.alert == true) "alerting" else "",
        if (contact.stale) "stale" else "",
    )
    return (position + trailing).filter { it.isNotBlank() }.joinToString("  ")
}
