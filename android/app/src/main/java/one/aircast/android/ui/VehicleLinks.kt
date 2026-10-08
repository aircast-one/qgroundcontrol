package one.aircast.android.ui

import one.aircast.map.optText
import org.json.JSONObject

internal const val VEHICLE_LINKS = "view.vehicleLinks"

internal data class VehicleLink(val commLost: Boolean?)

internal data class LossFailsafe(val action: String, val after: Double?)

internal data class VehicleLinks(
    val available: Boolean,
    val links: List<VehicleLink>,
    val contactLost: Boolean? = null,
    val failsafe: LossFailsafe? = null,
)

internal fun vehicleLinks(view: JSONObject?): VehicleLinks? {
    if (view == null || view.optText("class") != "VehicleLinks") return null
    val listed = view.optJSONArray("links")
    return VehicleLinks(
        available = view.optBoolean("available"),
        contactLost = if (view.isNull("contactLost")) null else view.optBoolean("contactLost"),
        failsafe = view.optText("lossAction").ifEmpty { null }?.let { LossFailsafe(it, if (view.isNull("lossAfter")) null else view.optDouble("lossAfter")) },
        links = (0 until (listed?.length() ?: 0)).mapNotNull { index ->
            listed?.optJSONObject(index)?.let { link ->
                VehicleLink(commLost = if (link.isNull("commLost")) null else link.optBoolean("commLost"))
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

internal fun linkNames(view: JSONObject?): List<String> {
    val listed = view?.optJSONArray("links") ?: return emptyList()
    return (0 until listed.length()).mapNotNull { index -> listed.optJSONObject(index)?.optText("name") }
}
