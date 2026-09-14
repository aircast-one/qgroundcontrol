package one.aircast.android.ui

import one.aircast.mapspike.optText
import org.json.JSONObject

internal const val VEHICLE_LINKS = "view.vehicleLinks"

internal data class VehicleLink(val name: String, val primary: Boolean, val commLost: Boolean?)

internal data class VehicleLinks(
    val available: Boolean,
    val watching: Boolean,
    val primary: String,
    val contactLost: Boolean?,
    val reason: String,
    val links: List<VehicleLink>,
)

internal fun vehicleLinks(view: JSONObject?): VehicleLinks? {
    if (view == null || view.optText("class") != "VehicleLinks") return null
    val listed = view.optJSONArray("links")
    return VehicleLinks(
        available = view.optBoolean("available"),
        watching = view.optBoolean("watching"),
        primary = view.optText("primary"),
        contactLost = if (view.isNull("contactLost")) null else view.optBoolean("contactLost"),
        reason = view.optText("reason"),
        links = (0 until (listed?.length() ?: 0)).mapNotNull { index ->
            listed?.optJSONObject(index)?.let { link ->
                VehicleLink(
                    name = link.optText("name"),
                    primary = link.optBoolean("primary"),
                    commLost = if (link.isNull("commLost")) null else link.optBoolean("commLost"),
                )
            }
        },
    )
}

internal data class LinkCell(val text: String, val degraded: Boolean)

internal fun linkCell(links: VehicleLinks?): LinkCell? {
    if (links == null || !links.available || links.links.size < 2) return null
    val silent = links.links.filter { it.commLost == true }
    return when {
        silent.isEmpty() -> LinkCell("${links.links.size} links", false)
        silent.size == links.links.size -> LinkCell("no link heard", true)
        silent.size == 1 -> LinkCell("1 link lost", true)
        else -> LinkCell("${silent.size} links lost", true)
    }
}
