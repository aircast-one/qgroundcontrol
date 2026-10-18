package one.aircast.android.ui

import one.aircast.map.optText
import org.json.JSONObject

internal data class BackendChoice(val status: DeviceBadge, val text: String, val switchTo: String, val label: String, val confirm: String, val success: String)

internal data class SignInMode(
    val value: String,
    val title: String,
    val desc: String,
    val fields: List<String>,
    val clears: List<String>,
    val submit: String,
    val unsaved: String,
    val confirm: String,
)

internal data class SignInField(val name: String, val label: String, val value: String, val placeholder: String, val secret: Boolean)

internal data class PublishSignIn(
    val status: DeviceBadge,
    val awaitingCloud: Boolean,
    val mode: String,
    val modes: List<SignInMode>,
    val fields: List<SignInField>,
    val environment: String,
    val verdict: DeviceBadge,
)

internal data class DiagnosticsSettings(
    val status: DeviceBadge,
    val hint: String,
    val endpoint: String,
    val token: String,
    val tokenLabel: String,
    val streaming: Boolean,
    val canStream: Boolean,
    val state: String,
    val confirm: String,
)

internal data class EngineeringView(val backend: BackendChoice, val publish: PublishSignIn, val diagnostics: DiagnosticsSettings)

private fun parsePublish(json: JSONObject?): PublishSignIn = PublishSignIn(
    status = parseBadge(json?.optJSONObject("status")),
    awaitingCloud = json?.optBoolean("awaitingCloud") == true,
    mode = json?.optText("mode").orEmpty(),
    modes = json.deviceObjects("modes").map {
        SignInMode(it.optText("value"), it.optText("title"), it.optText("desc"), it.deviceTexts("fields"), it.deviceTexts("clears"), it.optText("submit"), it.optText("unsaved"), it.optText("confirm"))
    },
    fields = json.deviceObjects("fields").map { SignInField(it.optText("name"), it.optText("label"), it.optText("value"), it.optText("placeholder"), it.optBoolean("secret")) },
    environment = json?.optText("environment").orEmpty(),
    verdict = parseBadge(json?.optJSONObject("verdict")),
)

private fun parseDiagnostics(json: JSONObject?): DiagnosticsSettings = DiagnosticsSettings(
    status = parseBadge(json?.optJSONObject("status")),
    hint = json?.optText("hint").orEmpty(),
    endpoint = json?.optText("endpoint").orEmpty(),
    token = json?.optText("token").orEmpty(),
    tokenLabel = json?.optText("tokenLabel").orEmpty(),
    streaming = json?.optBoolean("streaming") == true,
    canStream = json?.optBoolean("canStream") == true,
    state = json?.optText("state").orEmpty(),
    confirm = json?.optText("confirm").orEmpty(),
)

internal fun parseEngineering(json: JSONObject): EngineeringView? =
    json.optJSONObject("backend")?.let { backend ->
        EngineeringView(
            backend = BackendChoice(parseBadge(backend.optJSONObject("status")), backend.optText("text"), backend.optText("switch"), backend.optText("label"), backend.optText("confirm"), backend.optText("success")),
            publish = parsePublish(json.optJSONObject("publish")),
            diagnostics = parseDiagnostics(json.optJSONObject("diagnostics")),
        )
    }

internal fun modeNamed(publish: PublishSignIn, value: String): SignInMode? = publish.modes.firstOrNull { it.value == value }

internal fun signInFields(publish: PublishSignIn, value: String): List<SignInField> =
    modeNamed(publish, value)?.fields.orEmpty().let { names -> publish.fields.filter { it.name in names } }

internal fun signInBody(publish: PublishSignIn, value: String, edits: Map<String, String>): Map<String, String> =
    modeNamed(publish, value)?.clears.orEmpty().let { clears -> publish.fields.associate { it.name to if (it.name in clears) "" else edits[it.name] ?: it.value } }

internal fun signInKey(publish: PublishSignIn): String = (listOf(publish.mode) + publish.fields.map { it.value }).joinToString("|")
