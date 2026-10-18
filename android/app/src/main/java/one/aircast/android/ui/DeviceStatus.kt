package one.aircast.android.ui

import androidx.annotation.DrawableRes
import one.aircast.android.R
import one.aircast.map.optText
import org.json.JSONObject

internal data class DeviceAction(val label: String, val href: String)

internal data class DeviceCheck(val label: String, val value: String, val level: Int, val href: String, val action: String)

internal data class DevicePage(val title: String, val href: String, val icon: String, val summary: String, val needs: Boolean)

internal data class DeviceUplink(
    val carrier: String,
    val backup: String,
    val rate: String,
    val network: String,
    val standby: String,
    val trouble: String,
    val data: String,
    val charts: List<DeviceChartView>,
)

internal data class DeviceHealth(val memory: String, val warning: DeviceBadge?, val charts: List<DeviceChartView>)

internal data class DeviceStatus(
    val name: String,
    val level: Int,
    val headline: String,
    val detail: String,
    val action: DeviceAction?,
    val setup: DeviceAction?,
    val checks: List<DeviceCheck>,
    val pages: List<DevicePage>,
    val link: DeviceUplink?,
    val health: DeviceHealth?,
    val events: String,
)

internal fun parseDevicePage(json: JSONObject?): DevicePage =
    DevicePage(json?.optText("title").orEmpty(), json?.optText("href").orEmpty(), json?.optText("icon").orEmpty(), json?.optText("summary").orEmpty(), json?.optBoolean("needs") == true)

private fun parseCharts(json: JSONObject): List<DeviceChartView> = json.deviceObjects("charts").mapNotNull(::parseChart)

private fun parseUplink(json: JSONObject?): DeviceUplink? = json?.let {
    DeviceUplink(it.optText("carrier"), it.optText("backup"), it.optText("rate"), it.optText("network"), it.optText("standby"), it.optText("trouble"), it.optText("data"), parseCharts(it))
}

private fun parseHealth(json: JSONObject?): DeviceHealth? = json?.let {
    DeviceHealth(it.optText("memory"), it.optJSONObject("warning")?.let(::parseBadge), parseCharts(it))
}

private fun deviceAction(json: JSONObject?): DeviceAction? =
    json?.optText("href")?.takeIf { it.isNotEmpty() }?.let { DeviceAction(json.optText("label"), it) }

internal fun parseDeviceStatus(json: JSONObject): DeviceStatus? =
    json.takeIf { it.optText("headline").isNotEmpty() }?.let {
        DeviceStatus(
            name = it.optText("name"),
            level = it.optInt("level"),
            headline = it.optText("headline"),
            detail = it.optText("detail"),
            action = deviceAction(it.optJSONObject("action")),
            setup = deviceAction(it.optJSONObject("setup")),
            checks = it.deviceObjects("checks").map { check ->
                DeviceCheck(check.optText("label"), check.optText("value"), check.optInt("level"), check.optText("href"), check.optText("action"))
            },
            pages = it.deviceObjects("pages").map(::parseDevicePage),
            link = parseUplink(it.optJSONObject("link")),
            health = parseHealth(it.optJSONObject("health")),
            events = it.optText("events"),
        )
    }

private val PAGE_ICONS = mapOf(
    "qr-code" to R.drawable.ic_link,
    "video" to R.drawable.ic_videocam,
    "sliders-horizontal" to R.drawable.ic_tune,
    "network" to R.drawable.ic_lan,
    "activity" to R.drawable.ic_analytics,
    "gauge" to R.drawable.ic_stop_circle,
    "clock" to R.drawable.ic_history,
    "settings" to R.drawable.ic_developer_board,
    "server" to R.drawable.ic_terminal,
    "scan-search" to R.drawable.ic_build,
)

@DrawableRes
internal fun devicePageIcon(icon: String): Int = PAGE_ICONS[icon] ?: R.drawable.ic_settings

internal fun deviceNeedsAttention(status: DeviceStatus?): Boolean = status != null && status.level != DeviceLevel.Ready
