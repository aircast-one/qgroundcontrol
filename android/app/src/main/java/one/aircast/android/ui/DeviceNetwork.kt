package one.aircast.android.ui

import androidx.annotation.DrawableRes
import one.aircast.android.R
import one.aircast.map.optText
import org.json.JSONObject

internal data class NetworkFact(val label: String, val value: String)

internal data class DeviceProblem(val text: String, val detail: String)

internal data class ConfirmedAction(val label: String, val confirm: String)

internal data class UplinkBackup(val label: String, val status: DeviceBadge)

internal data class UplinkCard(
    val icon: String,
    val title: String,
    val status: DeviceBadge,
    val verdict: DeviceBadge,
    val detail: String,
    val addr: String,
    val meta: String,
    val warning: String,
    val rate: String,
    val failover: List<UplinkBackup>,
    val switchText: String,
    val backup: String,
)

internal data class SmsMessage(val from: String, val text: String, val time: String)

internal data class SmsInbox(val summary: String, val items: List<SmsMessage>)

internal data class SimSettings(val summary: String, val apn: String, val placeholder: String, val metered: String, val options: List<DeviceOption>)

internal data class ModemCard(
    val status: DeviceBadge,
    val summary: String,
    val notices: List<DeviceBadge>,
    val operator: String,
    val detail: String,
    val signal: DeviceChartView?,
    val facts: List<NetworkFact>,
    val messages: SmsInbox,
    val sim: SimSettings?,
)

internal data class StarlinkDemo(val on: Boolean, val note: String, val condition: String, val conditions: List<DeviceOption>, val first: Boolean)

internal data class StarlinkAddress(val value: String, val fallback: String, val defaultFirst: Boolean, val open: Boolean)

internal data class StarlinkCard(
    val status: DeviceBadge,
    val summary: String,
    val demo: StarlinkDemo?,
    val notices: List<DeviceBadge>,
    val pollError: String,
    val facts: List<NetworkFact>,
    val firmware: String,
    val address: StarlinkAddress,
)

internal data class WifiCard(val status: DeviceBadge, val summary: String, val joined: DeviceBadge?, val reopen: WifiReopen)

internal data class IPv4Settings(val method: String, val metric: String, val metricPlaceholder: String, val address: String, val gateway: String, val dns: String)

internal data class LinkEditor(val iface: String, val ipv4: IPv4Settings, val confirm: String, val uuid: String, val deactivateConfirm: String, val deleteConfirm: String)

internal data class NetLink(val name: String, val kind: String, val role: DeviceBadge?, val where: String, val local: Boolean, val usage: String, val editor: LinkEditor?)

internal data class SavedProfile(val uuid: String, val name: String, val kind: String, val ipv4: IPv4Settings, val activateConfirm: String, val deleteConfirm: String)

internal data class OtherUsage(val label: String, val month: String, val total: String)

internal data class NewConnection(val types: List<DeviceOption>, val ports: List<DeviceOption>, val autoconnect: List<DeviceOption>)

internal data class InterfacesCard(
    val status: DeviceBadge,
    val summary: String,
    val problem: DeviceProblem?,
    val links: List<NetLink>,
    val saved: List<SavedProfile>,
    val other: List<OtherUsage>,
    val methods: List<DeviceOption>,
    val add: NewConnection,
)

internal data class BudgetMeter(val label: String, val used: String, val percent: Int, val level: Int, val remaining: String, val note: String)

internal data class GaugeStat(val label: String, val value: String, val note: String)

internal data class DataGauge(
    val badges: List<DeviceBadge>,
    val hero: String,
    val heroCaption: String,
    val uplinkPercent: Int,
    val uplink: String,
    val downlink: String,
    val meters: List<BudgetMeter>,
    val stats: List<GaugeStat>,
)

internal data class BudgetSettings(val summary: String, val monthlyCapGB: String, val flightCapMB: String, val warnPercent: String)

internal data class DataCard(val status: DeviceBadge, val summary: String, val gauge: DataGauge?, val budget: BudgetSettings)

internal data class TailscaleControl(
    val title: String,
    val open: Boolean,
    val server: String,
    val authKeyPlaceholder: String,
    val authKeySet: Boolean,
    val switchError: String,
    val pending: String,
    val switchTo: ConfirmedAction?,
    val logOutToSwitch: ConfirmedAction?,
    val hint: String,
)

internal data class TailscaleBanner(val title: String, val subtitle: String, val status: DeviceBadge)

internal data class TailscaleSignIn(val label: String, val href: String)

internal data class TailscaleCard(
    val status: DeviceBadge,
    val summary: String,
    val problem: DeviceProblem?,
    val banner: TailscaleBanner?,
    val facts: List<NetworkFact>,
    val addresses: List<String>,
    val signIn: TailscaleSignIn?,
    val connect: String,
    val cancel: Boolean,
    val logOut: String,
    val hint: String,
    val control: TailscaleControl?,
)

internal data class IceServer(val url: String = "", val username: String = "", val password: String = "")

internal data class RelayCard(val status: DeviceBadge, val summary: String, val on: Boolean, val configured: Boolean, val reason: String, val notice: DeviceBadge, val servers: List<IceServer>)

internal data class NetworkView(
    val uplink: UplinkCard,
    val modem: ModemCard?,
    val starlink: StarlinkCard,
    val wifi: WifiCard,
    val interfaces: InterfacesCard,
    val data: DataCard,
    val tailscale: TailscaleCard,
    val relay: RelayCard,
)

private fun JSONObject.section(key: String): JSONObject = optJSONObject(key) ?: JSONObject()

private fun JSONObject.badge(key: String): DeviceBadge = parseBadge(optJSONObject(key))

internal fun optDeviceBadge(json: JSONObject?, key: String): DeviceBadge? = json?.optJSONObject(key)?.let(::parseBadge)

private fun parseFacts(json: JSONObject): List<NetworkFact> = json.deviceObjects("facts").map { NetworkFact(it.optText("label"), it.optText("value")) }

private fun parseProblem(json: JSONObject): DeviceProblem? = json.optJSONObject("problem")?.let { DeviceProblem(it.optText("text"), it.optText("detail")) }

private fun parseConfirmed(json: JSONObject?): ConfirmedAction? = json?.let { ConfirmedAction(it.optText("label"), it.optText("confirm")) }

private fun parseIPv4(json: JSONObject): IPv4Settings =
    IPv4Settings(json.optText("method"), json.optText("metric"), json.optText("metricPlaceholder"), json.optText("address"), json.optText("gateway"), json.optText("dns"))

private fun parseUplink(json: JSONObject): UplinkCard = UplinkCard(
    icon = json.optText("icon"),
    title = json.optText("title"),
    status = json.badge("status"),
    verdict = json.badge("verdict"),
    detail = json.optText("detail"),
    addr = json.optText("addr"),
    meta = json.optText("meta"),
    warning = json.optText("warning"),
    rate = json.optText("rate"),
    failover = json.deviceObjects("failover").map { UplinkBackup(it.optText("label"), it.badge("status")) },
    switchText = json.optText("switch"),
    backup = json.optText("backup"),
)

private fun parseSim(json: JSONObject?): SimSettings? =
    json?.let { SimSettings(it.optText("summary"), it.optText("apn"), it.optText("placeholder"), it.optText("metered"), parseOptions(it, "options")) }

private fun parseModem(json: JSONObject?): ModemCard? = json?.let {
    val messages = it.section("messages")
    ModemCard(
        status = it.badge("status"),
        summary = it.optText("summary"),
        notices = parseBadges(it, "notices"),
        operator = it.optText("operator"),
        detail = it.optText("detail"),
        signal = parseChart(it.optJSONObject("signal")),
        facts = parseFacts(it),
        messages = SmsInbox(messages.optText("summary"), messages.deviceObjects("items").map { item -> SmsMessage(item.optText("from"), item.optText("text"), item.optText("when")) }),
        sim = parseSim(it.optJSONObject("sim")),
    )
}

private fun parseDemo(json: JSONObject?): StarlinkDemo? =
    json?.let { StarlinkDemo(it.optBoolean("on"), it.optText("note"), it.optText("condition"), parseOptions(it, "conditions"), it.optBoolean("first")) }

private fun parseStarlink(json: JSONObject): StarlinkCard {
    val address = json.section("address")
    return StarlinkCard(
        status = json.badge("status"),
        summary = json.optText("summary"),
        demo = parseDemo(json.optJSONObject("demo")),
        notices = parseBadges(json, "notices"),
        pollError = json.optText("pollError"),
        facts = parseFacts(json),
        firmware = json.optText("firmware"),
        address = StarlinkAddress(address.optText("value"), address.optText("default"), address.optBoolean("defaultFirst"), address.optBoolean("open")),
    )
}

private fun parseWifi(json: JSONObject): WifiCard {
    val reopen = json.section("reopen")
    return WifiCard(json.badge("status"), json.optText("summary"), optDeviceBadge(json, "joined"), WifiReopen(reopen.optText("host"), reopen.optText("hostname")))
}

private fun parseEditor(json: JSONObject?): LinkEditor? = json?.let {
    LinkEditor(it.optText("interface"), parseIPv4(it.section("ipv4")), it.optText("confirm"), it.optText("uuid"), it.optText("deactivateConfirm"), it.optText("deleteConfirm"))
}

private fun parseInterfaces(json: JSONObject): InterfacesCard {
    val add = json.section("add")
    return InterfacesCard(
        status = json.badge("status"),
        summary = json.optText("summary"),
        problem = parseProblem(json),
        links = json.deviceObjects("links").map {
            NetLink(it.optText("name"), it.optText("kind"), optDeviceBadge(it, "role"), it.optText("where"), it.optBoolean("local"), it.optText("usage"), parseEditor(it.optJSONObject("editor")))
        },
        saved = json.deviceObjects("saved").map {
            SavedProfile(it.optText("uuid"), it.optText("name"), it.optText("kind"), parseIPv4(it.section("ipv4")), it.optText("activateConfirm"), it.optText("deleteConfirm"))
        },
        other = json.deviceObjects("other").map { OtherUsage(it.optText("label"), it.optText("month"), it.optText("total")) },
        methods = parseOptions(json, "methods"),
        add = NewConnection(parseOptions(add, "types"), parseOptions(add, "ports"), parseOptions(add, "autoconnect")),
    )
}

private fun parseGauge(json: JSONObject?): DataGauge? = json?.let {
    DataGauge(
        badges = parseBadges(it, "badges"),
        hero = it.optText("hero"),
        heroCaption = it.optText("heroCaption"),
        uplinkPercent = it.optInt("uplinkPercent"),
        uplink = it.optText("uplink"),
        downlink = it.optText("downlink"),
        meters = it.deviceObjects("meters").map { meter ->
            BudgetMeter(meter.optText("label"), meter.optText("used"), meter.optInt("percent"), meter.optInt("level"), meter.optText("remaining"), meter.optText("note"))
        },
        stats = it.deviceObjects("stats").map { stat -> GaugeStat(stat.optText("label"), stat.optText("value"), stat.optText("note")) },
    )
}

private fun parseData(json: JSONObject): DataCard {
    val budget = json.section("budget")
    return DataCard(
        json.badge("status"),
        json.optText("summary"),
        parseGauge(json.optJSONObject("gauge")),
        BudgetSettings(budget.optText("summary"), budget.optText("monthlyCapGB"), budget.optText("flightCapMB"), budget.optText("warnPercent")),
    )
}

private fun parseControl(json: JSONObject?): TailscaleControl? = json?.let {
    TailscaleControl(
        title = it.optText("title"),
        open = it.optBoolean("open"),
        server = it.optText("server"),
        authKeyPlaceholder = it.optText("authKeyPlaceholder"),
        authKeySet = it.optBoolean("authKeySet"),
        switchError = it.optText("switchError"),
        pending = it.optText("pending"),
        switchTo = parseConfirmed(it.optJSONObject("switch")),
        logOutToSwitch = parseConfirmed(it.optJSONObject("logOutToSwitch")),
        hint = it.optText("hint"),
    )
}

internal fun parseTailscale(json: JSONObject?): TailscaleCard {
    val card = json ?: JSONObject()
    return TailscaleCard(
        status = card.badge("status"),
        summary = card.optText("summary"),
        problem = parseProblem(card),
        banner = card.optJSONObject("banner")?.let { TailscaleBanner(it.optText("title"), it.optText("subtitle"), it.badge("status")) },
        facts = parseFacts(card),
        addresses = card.deviceTexts("addresses"),
        signIn = card.optJSONObject("signIn")?.let { TailscaleSignIn(it.optText("label"), it.optText("href")) },
        connect = card.optText("connect"),
        cancel = card.optBoolean("cancel"),
        logOut = card.optText("logOut"),
        hint = card.optText("hint"),
        control = parseControl(card.optJSONObject("control")),
    )
}

private fun parseRelay(json: JSONObject): RelayCard = RelayCard(
    status = json.badge("status"),
    summary = json.optText("summary"),
    on = json.optBoolean("on"),
    configured = json.optBoolean("configured"),
    reason = json.optText("reason"),
    notice = json.badge("notice"),
    servers = json.deviceObjects("servers").map { IceServer(it.optText("url"), it.optText("username"), it.optText("password")) },
)

internal fun parseNetwork(json: JSONObject): NetworkView? = json.optJSONObject("uplink")?.let { uplink ->
    NetworkView(
        uplink = parseUplink(uplink),
        modem = parseModem(json.optJSONObject("modem")),
        starlink = parseStarlink(json.section("starlink")),
        wifi = parseWifi(json.section("wifi")),
        interfaces = parseInterfaces(json.section("interfaces")),
        data = parseData(json.section("data")),
        tailscale = parseTailscale(json.optJSONObject("tailscale")),
        relay = parseRelay(json.section("relay")),
    )
}

private val UPLINK_ICONS = mapOf(
    "satellite-dish" to R.drawable.ic_satellite_alt,
    "signal" to R.drawable.ic_signal_cellular_alt,
    "wifi" to R.drawable.ic_wifi,
    "ethernet-port" to R.drawable.ic_settings_ethernet,
)

@DrawableRes
internal fun uplinkIcon(icon: String): Int = UPLINK_ICONS[icon] ?: R.drawable.ic_lan

internal const val GOOGLE_STUN = "stun:stun.l.google.com:19302"

internal fun iceRows(servers: List<IceServer>): List<IceServer> = servers.ifEmpty { listOf(IceServer()) }

internal fun serializeIceServers(rows: List<IceServer>): String =
    rows.map { IceServer(it.url.trim(), it.username.trim(), it.password.trim()) }
        .filter { it.url.isNotEmpty() }
        .joinToString("\n") { if (it.username.isNotEmpty() || it.password.isNotEmpty()) "${it.url} | ${it.username} | ${it.password}" else it.url }

internal fun manualAddressing(method: String): Boolean = method == "manual"

private val CONNECTION_FIELD_TYPES = mapOf("ssid" to "wifi", "psk" to "wifi", "apn" to "gsm", "pin" to "gsm")

internal fun connectionFieldShown(type: String, field: String): Boolean = CONNECTION_FIELD_TYPES[field]?.let { it == type } ?: true
