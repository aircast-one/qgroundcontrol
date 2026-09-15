package one.aircast.android.ui

import one.aircast.mapspike.optText
import org.json.JSONObject

internal const val FOLLOW_ME_VIEW = "view.followMe"

internal data class FollowMeReading(
    val mode: String,
    val enabled: Boolean,
    val wouldSend: Boolean,
    val reason: String,
    val following: Int,
    val vehicles: Int,
)

internal fun followMeReading(view: JSONObject?): FollowMeReading? {
    if (view == null || view.optText("class") != "FollowMe") return null
    val listed = view.optJSONArray("vehicles")
    val vehicles = (0 until (listed?.length() ?: 0)).mapNotNull { listed?.optJSONObject(it) }
    return FollowMeReading(
        mode = view.optText("mode"),
        enabled = view.optBoolean("enabled"),
        wouldSend = view.optBoolean("wouldSend"),
        reason = view.optText("reason"),
        following = vehicles.count { it.optBoolean("following") },
        vehicles = vehicles.size,
    )
}

internal fun followMeTrouble(reason: String): String = when (reason) {
    "modeUnknown" -> "the Follow Me setting is not one this app understands"
    "modeNever" -> "Follow Me is switched off"
    "noVehicles" -> "no vehicle is connected"
    "noVehicleInFollowMode" -> "no vehicle is in Follow Me mode"
    "noFix" -> "this phone has no position yet"
    "fixInvalid" -> "this phone's position is not valid"
    "fixStale" -> "this phone's position has stopped updating"
    "fixUnusable" -> "this phone's position cannot be used"
    "allVehiclesRefused" -> "the vehicle refused the position"
    else -> "the position is not being sent"
}

internal fun followMeAsked(mode: String): Boolean = mode == "always" || mode == "followMe"

internal fun followMeResting(reason: String): Boolean = reason == "noVehicleInFollowMode"

internal fun followMeLabel(reading: FollowMeReading?): String? {
    if (reading == null || !followMeAsked(reading.mode) || reading.vehicles == 0) return null
    if (followMeResting(reading.reason)) return null
    if (reading.wouldSend) {
        return when (reading.following) {
            0, 1 -> "Following you"
            else -> "Following you · ${reading.following} vehicles"
        }
    }
    return "Not following you — ${followMeTrouble(reading.reason)}"
}
