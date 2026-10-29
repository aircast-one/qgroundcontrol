package one.aircast.android.ui

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.map.optText
import org.json.JSONObject

internal data class PortOption(val value: String, val label: String, val address: String)

internal data class PiCamera(val note: String, val action: String)

internal data class CameraWizardView(
    val name: String,
    val title: String,
    val isNew: Boolean,
    val suggestedName: String,
    val rtsp: String,
    val rtspHint: String,
    val addressOpen: Boolean,
    val lookingFor: String,
    val checks: List<String>,
    val piCamera: PiCamera?,
    val iface: String,
    val interfaces: List<PortOption>,
    val transport: String,
    val transports: List<DeviceOption>,
    val coverage: String,
    val json: JSONObject,
)

internal data class WizardCamera(val host: String, val url: String, val needsAuth: Boolean, val pathFound: Boolean, val used: String, val note: String, val action: String)

internal data class WizardChip(val cidr: String, val text: String)

internal data class WizardPort(
    val iface: String,
    val label: String,
    val detail: String,
    val open: Boolean,
    val suggest: String,
    val hint: String,
    val go: String,
    val options: List<WizardChip>,
    val busy: Boolean,
    val status: String,
)

internal data class WizardMake(val id: String, val label: String, val sub: String, val action: String)

internal data class WizardFound(val n: Int, val refusal: String, val camera: WizardCamera?, val name: String)

internal data class WizardApply(val n: Int, val state: String, val text: String, val field: String, val url: String, val camera: String)

internal data class WizardNotice(val n: Int, val text: String)

internal data class DeviceWizard(
    val session: Long,
    val scanning: Boolean,
    val scans: Int,
    val again: Int,
    val adopt: WizardCamera?,
    val note: String,
    val checklist: String,
    val cams: List<WizardCamera>,
    val camHeading: String,
    val camsHint: String,
    val details: String,
    val todoHeading: String,
    val todo: List<WizardPort>,
    val rest: List<WizardPort>,
    val restTitle: String,
    val templates: List<WizardMake>,
    val templateBusy: Boolean,
    val templateNote: String,
    val template: WizardFound,
    val apply: WizardApply,
    val notice: WizardNotice,
)

internal data class WizardRefusal(val text: String, val warn: Boolean, val field: String?)

internal data class WizardNext(val kind: String, val rtsp: String, val name: String?, val camera: String, val refusal: WizardRefusal?, val dupAck: String, val iface: String, val ip: String)

internal data class WizardForm(val next: String, val skipsPort: Boolean, val applyNote: String)

internal const val PI_CAMERA = "rpiCamera"
internal const val DEVICE_WIZARD = "view.deviceWizard"
private const val WIZARD_OPEN = "deviceWizard.open"
private const val WIZARD_SCAN = "deviceWizard.scan"
private const val WIZARD_FIX_PORT = "deviceWizard.fixPort"
private const val WIZARD_TEMPLATE = "deviceWizard.template"
private const val WIZARD_NEXT = "deviceWizard.next"
private const val WIZARD_FORM = "deviceWizard.form"
private const val WIZARD_SIGN_IN = "deviceWizard.signIn"
private const val WIZARD_APPLY = "deviceWizard.apply"
private const val WIZARD_RESTART = "deviceWizard.restart"

internal val STEP_NAMES = listOf("Camera", "Port", "Picture")

internal val NO_FORM = WizardForm("Continue", false, "")

internal fun parseCameraWizard(json: JSONObject?): CameraWizardView? = json?.let {
    CameraWizardView(
        name = it.optText("name"),
        title = it.optText("title"),
        isNew = it.optBoolean("new"),
        suggestedName = it.optText("suggestedName"),
        rtsp = it.optText("rtsp"),
        rtspHint = it.optText("rtspHint"),
        addressOpen = it.optBoolean("addressOpen"),
        lookingFor = it.optText("lookingFor"),
        checks = it.optJSONArray("checks")?.let { checks -> (0 until checks.length()).map(checks::optString) }.orEmpty(),
        piCamera = it.optJSONObject("piCamera")?.let { pi -> PiCamera(pi.optText("note"), pi.optText("action")) },
        iface = it.optText("interface"),
        interfaces = it.deviceObjects("interfaces").map { port -> PortOption(port.optText("value"), port.optText("label"), port.optText("address")) },
        transport = it.optText("transport"),
        transports = parseOptions(it, "transports"),
        coverage = it.optText("coverage"),
        json = it,
    )
}

private fun parseWizardCamera(json: JSONObject?): WizardCamera? = json?.let {
    WizardCamera(it.optText("host"), it.optText("url"), it.optBoolean("needsAuth"), it.optBoolean("pathFound"), it.optText("used"), it.optText("note"), it.optText("action"))
}

private fun parseWizardPort(json: JSONObject): WizardPort =
    WizardPort(
        iface = json.optText("iface"),
        label = json.optText("label"),
        detail = json.optText("detail"),
        open = json.optBoolean("open"),
        suggest = json.optText("suggest"),
        hint = json.optText("hint"),
        go = json.optText("go"),
        options = json.deviceObjects("options").map { WizardChip(it.optText("cidr"), it.optText("text")) },
        busy = json.optBoolean("busy"),
        status = json.optText("status"),
    )

internal fun parseDeviceWizard(json: JSONObject?): DeviceWizard {
    val template = json?.optJSONObject("template")
    val apply = json?.optJSONObject("apply")
    val notice = json?.optJSONObject("notice")
    return DeviceWizard(
        session = json?.optLong("session", -1L) ?: -1L,
        scanning = json?.optBoolean("scanning") == true,
        scans = json?.optInt("scans") ?: 0,
        again = json?.optInt("again") ?: 0,
        adopt = parseWizardCamera(json?.optJSONObject("adopt")),
        note = json?.optText("note").orEmpty(),
        checklist = json?.optText("checklist").orEmpty(),
        cams = json.deviceObjects("cams").mapNotNull(::parseWizardCamera),
        camHeading = json?.optText("camHeading").orEmpty(),
        camsHint = json?.optText("camsHint").orEmpty(),
        details = json?.optText("details").orEmpty(),
        todoHeading = json?.optText("todoHeading").orEmpty(),
        todo = json.deviceObjects("todo").map(::parseWizardPort),
        rest = json.deviceObjects("rest").map(::parseWizardPort),
        restTitle = json?.optText("restTitle").orEmpty(),
        templates = json.deviceObjects("templates").map { WizardMake(it.optText("id"), it.optText("label"), it.optText("sub"), it.optText("action")) },
        templateBusy = json?.optBoolean("templateBusy") == true,
        templateNote = json?.optText("templateNote").orEmpty(),
        template = WizardFound(template?.optInt("n") ?: 0, template?.optText("refusal").orEmpty(), parseWizardCamera(template?.optJSONObject("camera")), template?.optText("name").orEmpty()),
        apply = WizardApply(
            apply?.optInt("n") ?: 0,
            apply?.optText("state").orEmpty(),
            apply?.optText("text").orEmpty(),
            apply?.optText("field").orEmpty(),
            apply?.optText("url").orEmpty(),
            apply?.optText("camera").orEmpty(),
        ),
        notice = WizardNotice(notice?.optInt("n") ?: 0, notice?.optText("text").orEmpty()),
    )
}

internal fun parseWizardNext(json: JSONObject?): WizardNext? = json?.let {
    WizardNext(
        kind = it.optText("kind"),
        rtsp = it.optText("rtsp"),
        name = it.takeUnless { next -> next.isNull("name") }?.optText("name"),
        camera = it.optText("camera"),
        refusal = it.optJSONObject("refusal")?.let { refused -> WizardRefusal(refused.optText("text"), refused.optBoolean("warn"), refused.takeUnless { r -> r.isNull("field") }?.optText("field")) },
        dupAck = it.optText("dupAck"),
        iface = it.optText("iface"),
        ip = it.optText("ip"),
    )
}

internal fun parseWizardForm(json: JSONObject?): WizardForm = json?.let { WizardForm(it.optText("next"), it.optBoolean("skipsPort"), it.optText("applyNote")) } ?: NO_FORM

internal fun continueLeads(rtsp: String): Boolean = rtsp.isNotBlank()

internal fun repairing(view: CameraWizardView): Boolean = !view.isNew && view.lookingFor.isNotEmpty()

internal fun nextOffered(view: CameraWizardView, rtsp: String, picked: Boolean): Boolean = !repairing(view) || picked || rtsp.isNotBlank()

internal fun scanLeads(rtsp: String, picked: Boolean): Boolean = rtsp.isBlank() && !picked

private suspend fun wizardCall(path: String, vararg args: Any?): JSONObject? = withContext(Dispatchers.Default) { Qgc.call(path, *args) }

private suspend fun wizardResult(path: String, vararg args: Any?): JSONObject? = wizardCall(path, *args)?.optJSONObject("result")

internal suspend fun openWizard(view: CameraWizardView, scan: Boolean): Long = wizardResult(WIZARD_OPEN, view.json, scan)?.optLong("session", -1L) ?: -1L

internal suspend fun scanForCameras() {
    wizardCall(WIZARD_SCAN)
}

internal suspend fun fixWizardPort(iface: String, address: String) {
    wizardCall(WIZARD_FIX_PORT, iface, address)
}

internal suspend fun setUpCameraMake(id: String) {
    wizardCall(WIZARD_TEMPLATE, id)
}

internal suspend fun wizardNext(rtsp: String, name: String, dupAck: String): WizardNext? = parseWizardNext(wizardResult(WIZARD_NEXT, rtsp, name, dupAck))

internal suspend fun wizardForm(rtsp: String, iface: String, ip: String, current: String): WizardForm = parseWizardForm(wizardResult(WIZARD_FORM, rtsp, iface, ip, current))

internal suspend fun signedInUrl(base: String, user: String, pass: String): String = wizardCall(WIZARD_SIGN_IN, base, user, pass)?.optText("result") ?: base

internal suspend fun applyCamera(rtsp: String, camera: String, iface: String, ip: String, transport: String) {
    wizardCall(WIZARD_APPLY, rtsp, camera, iface, ip, transport)
}

internal suspend fun restartWizard() {
    wizardCall(WIZARD_RESTART)
}
