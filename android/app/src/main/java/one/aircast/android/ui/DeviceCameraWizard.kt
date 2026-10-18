package one.aircast.android.ui

import one.aircast.map.optText
import org.json.JSONObject
import java.net.URI

internal data class PortOption(val value: String, val label: String, val address: String)

internal data class CameraRef(val name: String, val source: String)

internal data class CameraTemplate(
    val id: String,
    val label: String,
    val hint: String,
    val cidr: String,
    val host: String,
    val name: String,
    val gimbal: String,
    val gimbalAddr: String,
)

internal data class CameraSubnet(val cidr: String, val label: String)

internal data class PiCamera(val note: String, val action: String)

internal data class CameraWizardView(
    val name: String,
    val isNew: Boolean,
    val suggestedName: String,
    val source: String,
    val live: Boolean,
    val rtsp: String,
    val rtspHint: String,
    val addressOpen: Boolean,
    val lookingFor: String,
    val piCamera: PiCamera?,
    val iface: String,
    val interfaces: List<PortOption>,
    val transport: String,
    val transports: List<DeviceOption>,
    val others: List<CameraRef>,
    val templates: List<CameraTemplate>,
    val subnets: List<CameraSubnet>,
    val coverage: String,
)

internal data class ScanNeighbor(val addr: String, val suggest: String)

internal data class ScanPort(
    val iface: String,
    val label: String,
    val type: String,
    val state: String,
    val subnet: String,
    val checked: Int,
    val reason: String,
    val triedAddress: String,
    val isUplink: Boolean,
    val neighbors: List<ScanNeighbor>,
)

internal data class FoundCamera(val host: String, val url: String, val needsAuth: Boolean, val pathFound: Boolean, val used: String)

internal data class ScannedPort(val port: ScanPort, val found: Boolean)

internal data class ScanOutcome(val cams: List<FoundCamera>, val ports: List<ScanPort>, val todo: List<ScannedPort>, val rest: List<ScannedPort>, val note: String)

internal data class PortFixer(val open: Boolean, val input: String, val suggest: String, val status: String, val busy: Boolean)

internal data class CameraCheck(val state: String, val reason: String)

internal const val PI_CAMERA = "rpiCamera"

internal const val SCANNING_NOTE = "Looking for cameras on every port of this device — up to 20 seconds…"
internal const val SCAN_FAILED_NOTE = "Couldn't reach this device to run the scan — check you're still connected to it, or type the camera's address below."
internal const val NO_ANSWER_CHECKLIST = "No camera answered — check it, then scan again"

private val URL_SCHEME = Regex("^[a-z][a-z0-9+.-]*://", RegexOption.IGNORE_CASE)
private val BARE_ADDRESS = Regex("[\\w.-]+(:\\d+)?(/.*)?")
private val TYPED_CIDR = Regex("(?:rtsps?://)?(\\d{1,3}(?:\\.\\d{1,3}){3})/(\\d|[12]\\d|3[0-2])", RegexOption.IGNORE_CASE)
private val FOUR_OCTETS = Regex("(\\d+)\\.(\\d+)\\.(\\d+)\\.(\\d+)")
private val DIGITS = Regex("\\d+")
private val RTSP_SCHEME = Regex("^rtsps?://", RegexOption.IGNORE_CASE)
private val RTSP_URL = Regex("^rtsps?://.+", RegexOption.IGNORE_CASE)
private val LAST_OCTET = Regex("\\.\\d+$")

private fun parseSubnets(json: JSONObject): List<CameraSubnet> = json.deviceObjects("subnets").map { CameraSubnet(it.optText("cidr"), it.optText("label")) }

internal fun parseCameraWizard(json: JSONObject?): CameraWizardView? = json?.let {
    CameraWizardView(
        name = it.optText("name"),
        isNew = it.optBoolean("new"),
        suggestedName = it.optText("suggestedName"),
        source = it.optText("source"),
        live = it.optBoolean("live"),
        rtsp = it.optText("rtsp"),
        rtspHint = it.optText("rtspHint"),
        addressOpen = it.optBoolean("addressOpen"),
        lookingFor = it.optText("lookingFor"),
        piCamera = it.optJSONObject("piCamera")?.let { pi -> PiCamera(pi.optText("note"), pi.optText("action")) },
        iface = it.optText("interface"),
        interfaces = it.deviceObjects("interfaces").map { port -> PortOption(port.optText("value"), port.optText("label"), port.optText("address")) },
        transport = it.optText("transport"),
        transports = parseOptions(it, "transports"),
        others = it.deviceObjects("others").map { other -> CameraRef(other.optText("name"), other.optText("source")) },
        templates = it.deviceObjects("templates").map { template ->
            CameraTemplate(
                template.optText("id"),
                template.optText("label"),
                template.optText("hint"),
                template.optText("cidr"),
                template.optText("host"),
                template.optText("name"),
                template.optText("gimbal"),
                template.optText("gimbalAddr"),
            )
        },
        subnets = parseSubnets(it),
        coverage = it.optText("coverage"),
    )
}

internal fun parseScanPorts(json: JSONObject?): List<ScanPort> =
    json.deviceObjects("ports").map {
        ScanPort(
            iface = it.optText("interface"),
            label = it.optText("label"),
            type = it.optText("type"),
            state = it.optText("state"),
            subnet = it.optText("subnet"),
            checked = it.optInt("checked"),
            reason = it.optText("reason"),
            triedAddress = it.optText("triedAddress"),
            isUplink = it.optBoolean("isUplink"),
            neighbors = it.deviceObjects("neighbors").map { neighbor -> ScanNeighbor(neighbor.optText("addr"), neighbor.optText("suggest")) },
        )
    }

internal fun parseFoundCameras(json: JSONObject?, wizardName: String): List<FoundCamera> =
    json.deviceObjects("cameras").map {
        FoundCamera(it.optText("host"), it.optText("url"), it.optBoolean("needsAuth"), it.optBoolean("pathFound"), it.optText("usedBy").takeIf { used -> used != wizardName }.orEmpty())
    }

internal fun parseCameraStatus(json: JSONObject?): CameraCheck? = json?.let { CameraCheck(it.optText("state"), it.optText("reason")) }

internal fun rtspHost(rtsp: String): String = runCatching { URI(rtsp).host }.getOrNull().orEmpty()

internal fun suggestIP(rtsp: String): String =
    FOUR_OCTETS.matchEntire(rtspHost(rtsp))?.groupValues?.let { (_, a, b, c, d) -> "$a.$b.$c.${if (d == "2") "3" else "2"}/24" }.orEmpty()

internal fun normalizeRtsp(raw: String): String {
    val value = raw.trim()
    return if (value.isEmpty() || URL_SCHEME.containsMatchIn(value) || !BARE_ADDRESS.matches(value)) value else "rtsp://$value"
}

internal fun cidrIn(rtsp: String): String = TYPED_CIDR.matchEntire(rtsp.trim())?.groupValues?.let { "${it[1]}/${it[2]}" }.orEmpty()

internal fun ipToInt(ip: String): Long? =
    ip.split('.').takeIf { it.size == 4 }?.fold<String, Long?>(0L) { total, part ->
        total?.let { sum -> part.takeIf(DIGITS::matches)?.toLongOrNull()?.takeIf { it <= 255 }?.let { octet -> sum * 256 + octet } }
    }

internal fun sameSubnet(host: String, subnet: String): Boolean {
    val bits = subnet.split('/').getOrNull(1)?.toIntOrNull()?.takeIf { it in 0..32 } ?: return false
    val a = ipToInt(host) ?: return false
    val b = ipToInt(subnet.substringBefore('/')) ?: return false
    val mask = if (bits == 0) 0L else (0xffffffffL shl (32 - bits)) and 0xffffffffL
    return (a and mask) == (b and mask)
}

internal fun portHasSubnet(port: ScanPort, want: String): Boolean {
    val host = want.substringBefore('/')
    return port.subnet.isNotEmpty() && host.isNotEmpty() && sameSubnet(host, port.subnet)
}

internal fun firstSuggest(port: ScanPort): String = port.neighbors.map { it.suggest }.firstOrNull { it.isNotEmpty() }.orEmpty()

internal fun subnetName(subnets: List<CameraSubnet>, subnet: String): String =
    if (subnet.isEmpty()) "" else subnets.firstOrNull { sameSubnet(it.cidr.substringBefore('/'), subnet) }?.label.orEmpty()

internal fun idleOnCameraSubnet(subnets: List<CameraSubnet>, port: ScanPort): Boolean =
    subnetName(subnets, port.subnet).isNotEmpty() && port.neighbors.isNotEmpty() && firstSuggest(port).isEmpty()

internal fun camRowLabel(cam: FoundCamera): String = when {
    !cam.pathFound -> "answers on RTSP — add the stream path"
    cam.needsAuth -> "needs a username and password"
    else -> "ready to use"
}

internal fun rowNote(cam: FoundCamera): String = if (cam.used.isNotEmpty()) "already added as ${cam.used}" else camRowLabel(cam)

internal fun neighborLabel(port: ScanPort): String {
    val lead = port.neighbors.firstOrNull() ?: return ""
    val more = if (port.neighbors.size > 1) " (and ${port.neighbors.size - 1} more)" else ""
    val here = if (port.subnet.isNotEmpty()) ", not on ${port.subnet}" else ""
    val advice = if (lead.suggest.isNotEmpty()) " — set this port to ${lead.suggest} to reach it" else " — probably a router or switch, not a camera"
    return "a device answered at ${lead.addr}$more (not confirmed as a camera)$here$advice"
}

private val REASONS: Map<String, (String) -> String> = mapOf(
    "address-unusable" to { address -> "$address could not be used — another device on this network already has it, or nothing on that subnet answered" },
    "address-expired" to { address -> "$address expired and could not be renewed" },
    "needs-password" to { _ -> "this port needs a password before it can connect" },
    "config-failed" to { _ -> "the settings on this port were refused — set an address below" },
)

private fun stateLabel(port: ScanPort): String = when (port.state) {
    "searched" -> "checked ${port.checked} addresses on ${port.subnet} — nothing answered"
    "no-carrier" -> if (port.type == "wi-fi") "not connected to any network" else "nothing plugged in — check the cable and the camera's power"
    "not-configured" -> "cable is in, but this port has no address yet"
    "configuring" -> "still getting an address — try again in a moment"
    "failed" -> "could not get an address — set one below"
    "no-address" -> "no address on this port — set one below to reach the camera"
    "subnet-too-big" -> "${port.subnet} is too wide to sweep — type the camera's address below"
    "no-hosts" -> "${port.subnet} has no other address on it"
    else -> port.state
}

internal fun portLabel(port: ScanPort): String =
    neighborLabel(port).ifEmpty { REASONS[port.reason]?.invoke(port.triedAddress.ifEmpty { "that address" }) ?: stateLabel(port) }

internal fun cableLabel(port: ScanPort, host: String): String =
    if (host.isNotEmpty() && port.subnet.isNotEmpty() && sameSubnet(host, port.subnet)) {
        "nothing answered at $host on ${port.iface} — the port is set right; check the cable goes to the camera, not a router or switch."
    } else {
        ""
    }

internal fun portActionable(port: ScanPort, found: Boolean): Boolean = !found && !port.isUplink && (port.neighbors.isNotEmpty() || port.state != "searched")

internal fun portDetail(scanned: ScannedPort, subnets: List<CameraSubnet>, sourceHost: String): String {
    val port = scanned.port
    return when {
        scanned.found -> "camera found here"
        port.isUplink -> "${portLabel(port)} — this is the device's own uplink"
        idleOnCameraSubnet(subnets, port) ->
            "Set for ${subnetName(subnets, port.subnet)} (${port.subnet}). Something at ${port.neighbors.firstOrNull()?.addr.orEmpty()} is on this cable — probably a router or switch, not a camera. Plug the camera in here and scan again."
        else -> cableLabel(port, sourceHost).ifEmpty { portLabel(port) }
    }
}

internal fun scanOutcome(cams: List<FoundCamera>, ports: List<ScanPort>, sourceHost: String): ScanOutcome {
    val fresh = cams.filter { it.used.isEmpty() }
    val (todo, rest) = ports.map { port ->
        val found = cams.any { port.subnet.isNotEmpty() && sameSubnet(it.host, port.subnet) }
        val wired = cableLabel(port, sourceHost).isNotEmpty() && fresh.isEmpty()
        ScannedPort(port, found) to (wired || portActionable(port, found || fresh.isNotEmpty()))
    }.partition { it.second }
    val note = when {
        cams.isNotEmpty() -> ""
        ports.isNotEmpty() -> "No camera answered."
        else -> "No camera port on this device — type the camera's address below."
    }
    return ScanOutcome(cams, ports, todo.map { it.first }, rest.map { it.first }, note)
}

internal fun fixersFor(outcome: ScanOutcome, subnets: List<CameraSubnet>): Map<String, PortFixer> =
    outcome.todo.associate { (port) ->
        val suggest = firstSuggest(port)
        port.iface to PortFixer(open = suggest.isNotEmpty() || (outcome.todo.size == 1 && !idleOnCameraSubnet(subnets, port)), input = suggest, suggest = suggest, status = "", busy = false)
    }

internal fun adoptedCamera(outcome: ScanOutcome, auto: Boolean, rtsp: String): FoundCamera? =
    outcome.cams.filter { it.used.isEmpty() }.singleOrNull()?.takeIf { it.pathFound && !(auto && rtsp.trim().isNotEmpty()) }

internal fun camHeading(cams: List<FoundCamera>): String = if (cams.size == 1) "1 camera" else "${cams.size} cameras"

internal fun checklistHint(ports: List<ScanPort>, subnets: List<CameraSubnet>): String {
    val dead = ports.firstOrNull { it.type == "ethernet" && it.state == "no-carrier" }
    val idle = ports.firstOrNull { idleOnCameraSubnet(subnets, it) }
    return when {
        dead != null -> "Nothing is plugged into the ${dead.label}."
        idle != null -> "Something else is plugged into the ${idle.label} — it looks like a router or switch, not a camera."
        else -> ""
    }
}

internal fun todoHeading(todo: List<ScannedPort>, subnets: List<CameraSubnet>): String {
    val only = todo.singleOrNull() ?: return "Ports you can still set up"
    return if (idleOnCameraSubnet(subnets, only.port)) "Check the cable" else "Try this"
}

internal fun subnetOptions(port: ScanPort, ports: List<ScanPort>, subnets: List<CameraSubnet>): List<CameraSubnet> {
    val found = port.neighbors.filter { it.suggest.isNotEmpty() }.map { CameraSubnet(it.suggest, "found ${it.addr}") }
    val tried = listOfNotNull(port.triedAddress.takeIf { it.isNotEmpty() }?.let { CameraSubnet(it, "tried before") })
    return (found + tried + subnets).filter { option -> ports.none { it.iface != port.iface && portHasSubnet(it, option.cidr) } }.distinctBy { it.cidr }
}

internal fun fixerHint(fixer: PortFixer): String =
    if (fixer.suggest.isNotEmpty()) {
        "This device's address on the camera's subnet. Prefilled from the device that answered on this port."
    } else {
        "This device's address on the camera's subnet — match the camera (a 192.168.144.25 camera needs 192.168.144.x here). Pick your camera's make below, or read its address from its app or manual."
    }

internal fun fixerGoLabel(port: ScanPort, fixer: PortFixer): String = if (fixer.suggest.isNotEmpty()) "Set up ${port.iface} and find the camera" else "Set up & scan again"

internal fun templatePort(ports: List<ScanPort>, template: CameraTemplate): ScanPort? =
    ports.firstOrNull { !it.isUplink && portHasSubnet(it, template.cidr) }
        ?: ports.firstOrNull { !it.isUplink && it.type == "ethernet" && it.state != "no-carrier" }
        ?: ports.firstOrNull { !it.isUplink && it.type == "ethernet" }

internal fun templateReadyPort(template: CameraTemplate, cams: List<FoundCamera>, ports: List<ScanPort>): ScanPort? =
    if (cams.any { it.host == template.host }) null else ports.firstOrNull { portHasSubnet(it, template.cidr) }

internal fun templateSub(template: CameraTemplate, ready: ScanPort?): String =
    ready?.let { "The ${it.label} is ready for a ${template.label} — nothing answered at ${template.host}. Check the cable." } ?: template.hint

internal fun templateAction(busy: Boolean, ready: ScanPort?): String = when {
    busy -> "Setting up…"
    ready != null -> "Scan again"
    else -> "Set up"
}

internal fun templateHolderError(holder: ScanPort, template: CameraTemplate): String =
    "The ${holder.label} already uses ${template.cidr.substringBefore('/').replace(LAST_OCTET, ".x")}, so the camera port cannot take it too — move the ${template.label} to another subnet in its app, or disconnect ${holder.label} first."

internal fun camURL(base: String, user: String, pass: String): String =
    if (base.isEmpty() || (user.isBlank() && pass.isEmpty())) {
        base
    } else {
        RTSP_SCHEME.replace(base) { "${it.value}${uriComponent(user.trim())}${if (pass.isNotEmpty()) ":${uriComponent(pass)}" else ""}@" }
    }

internal fun reachablePort(rtsp: String, ports: List<ScanPort>): ScanPort? {
    val host = rtspHost(rtsp).ifEmpty { return null }
    return ports.firstOrNull { !it.isUplink && it.subnet.isNotEmpty() && sameSubnet(host, it.subnet) }
}

internal fun commitsNow(rtsp: String, ports: List<ScanPort>): Boolean = rtsp.isNotBlank() && (rtsp.trim() == PI_CAMERA || reachablePort(rtsp.trim(), ports) != null)

internal fun nextLabel(rtsp: String, ports: List<ScanPort>, isNew: Boolean): String = when {
    !commitsNow(rtsp, ports) -> "Continue"
    isNew -> "Add camera"
    else -> "Set up & verify"
}

internal fun continueLeads(rtsp: String): Boolean = rtsp.isNotBlank()

internal fun scanLeads(rtsp: String, picked: Boolean): Boolean = rtsp.isBlank() && !picked

internal data class NextInput(val rtsp: String, val name: String?, val others: List<CameraRef>, val ports: List<ScanPort>, val dupAck: String)

internal data class WizardRefusal(val text: String, val warn: Boolean, val field: String?)

internal sealed interface NextOutcome {
    val rtsp: String
    val name: String?

    data class Refuse(override val rtsp: String, override val name: String?, val refusal: WizardRefusal, val dupAck: String) : NextOutcome
    data class Apply(override val rtsp: String, override val name: String?) : NextOutcome
    data class Port(override val rtsp: String, override val name: String?) : NextOutcome
}

internal fun nextStep(input: NextInput): NextOutcome {
    fun refuse(rtsp: String, name: String?, text: String, field: String?, warn: Boolean = false, ack: String = input.dupAck): NextOutcome =
        NextOutcome.Refuse(rtsp, name, WizardRefusal(text, warn, field), ack)
    val cidr = cidrIn(input.rtsp)
    if (cidr.isNotEmpty()) {
        return refuse(
            input.rtsp,
            input.name,
            "$cidr is an address for this device's port, not the camera. It belongs in the port box above — set the port to it, and the scan fills this field with the camera's own URL.",
            "rtsp",
        )
    }
    val piCamera = input.rtsp.trim() == PI_CAMERA
    val rtsp = if (piCamera) PI_CAMERA else normalizeRtsp(input.rtsp)
    val name = input.name?.trim()?.lowercase()
    val duplicate = input.others.firstOrNull { it.source.trim() == rtsp }
    return when {
        name == "" -> refuse(rtsp, name, "Give the camera a name.", "name")
        name != null && input.others.any { it.name.lowercase() == name } -> refuse(rtsp, name, "A camera named “$name” already exists — pick another name.", "name")
        !piCamera && !RTSP_URL.containsMatchIn(rtsp) -> refuse(rtsp, name, "Enter the camera's address, e.g. 192.168.8.10 or rtsp://192.168.8.10:554/stream", "rtsp")
        duplicate != null && input.dupAck != rtsp -> refuse(
            rtsp,
            name,
            "Camera “${duplicate.name}” already uses this source. If that camera only allows one connection, this one won't get video. Click Continue again to add it anyway, or change the URL.",
            null,
            warn = true,
            ack = rtsp,
        )
        piCamera || reachablePort(rtsp, input.ports) != null -> NextOutcome.Apply(rtsp, name)
        else -> NextOutcome.Port(rtsp, name)
    }
}

internal fun portToSuggest(ports: List<ScanPort>): String = ports.firstOrNull { !it.isUplink && it.state != "searched" }?.iface.orEmpty()

internal fun applyNote(iface: String, ip: String, current: String): String = when {
    iface.isEmpty() || ip.isBlank() -> ""
    current == ip.trim() -> "$iface keeps ${ip.trim()} — nothing on that port changes."
    else -> "Sets $iface to ${ip.trim()} — anything else on that port may need reconnecting."
}

internal val STEP_NAMES = listOf("Camera", "Port", "Picture")
