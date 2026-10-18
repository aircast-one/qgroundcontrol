package one.aircast.android.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.staticCompositionLocalOf
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcPath
import one.aircast.map.optText
import org.json.JSONObject
import java.net.URI

internal object DeviceLevel {
    const val Ready = 0
    const val Warn = 1
    const val Blocked = 2
    const val Neutral = 3
}

internal const val DEVICE_PAGE = "Aircast device"
internal const val DEVICE_CONNECT = "device.connect"
internal const val DEVICE_DISCONNECT = "device.disconnect"
private const val DEVICE_CALL = "device.call"
private const val DEVICE_READ = "device.read"
private const val DEVICE_UPLOAD = "device.upload"
private const val DEVICE_QR = "device.qr"
private const val DEVICE_REACH = "device.reach"

internal fun deviceViewPath(page: String): String = "view.device($page)"

internal fun uriComponent(text: String): String = java.net.URLEncoder.encode(text, "UTF-8").replace("+", "%20")

internal data class DeviceReading(val origin: String, val host: String, val answered: Boolean, val view: JSONObject?)

internal fun deviceReading(json: JSONObject?): DeviceReading =
    DeviceReading(json?.optText("origin").orEmpty(), json?.optText("host").orEmpty(), json?.optBoolean("answered") == true, json?.optJSONObject("view"))

@Composable
internal fun deviceReading(page: String): DeviceReading {
    val json by qgcPath(deviceViewPath(page))
    return remember(json) { deviceReading(json) }
}

@Composable
internal fun <T> deviceView(page: String, parse: (JSONObject) -> T?): T? {
    val json by qgcPath(deviceViewPath(page))
    val reader by rememberUpdatedState(parse)
    return remember(json) { deviceReading(json).view?.let(reader) }
}

internal val LocalDeviceOrigin = staticCompositionLocalOf { "" }

internal fun resolveDeviceUrl(origin: String, path: String): String =
    if (origin.isEmpty()) path else runCatching { URI(origin).resolve(path).toString() }.getOrDefault(path)

@Composable
internal fun deviceUrl(path: String): String = resolveDeviceUrl(LocalDeviceOrigin.current, path)

internal data class DeviceRefusal(val message: String, val field: String)

internal enum class DeviceMethod { POST, PUT, DELETE }

internal val NO_ANSWER = DeviceRefusal("The device did not answer.", "")

internal fun deviceRefusal(answer: JSONObject?): DeviceRefusal? = when {
    answer == null -> NO_ANSWER
    answer.optBoolean("ok") -> null
    else -> DeviceRefusal(answer.optText("reason").ifBlank { NO_ANSWER.message }, answer.optText("field"))
}

internal data class DeviceAnswer(val refusal: DeviceRefusal?, val result: JSONObject?)

internal fun deviceAnswer(answer: JSONObject?): DeviceAnswer = DeviceAnswer(deviceRefusal(answer), answer?.optJSONObject("result"))

internal suspend fun deviceSend(method: DeviceMethod, path: String, body: JSONObject? = null): DeviceAnswer =
    withContext(Dispatchers.Default) { deviceAnswer(Qgc.call(DEVICE_CALL, method.name, path, body ?: JSONObject.NULL)) }

internal suspend fun deviceCall(method: DeviceMethod, path: String, body: JSONObject? = null): DeviceRefusal? = deviceSend(method, path, body).refusal

internal data class DevicePart(val name: String, val file: java.io.File? = null, val value: String = "")

internal fun devicePartJson(part: DevicePart): JSONObject =
    JSONObject().put("name", part.name).apply { part.file?.let { put("file", it.absolutePath) } ?: put("value", part.value) }

internal suspend fun deviceUpload(method: DeviceMethod, path: String, parts: List<DevicePart>): DeviceAnswer =
    withContext(Dispatchers.Default) { deviceAnswer(Qgc.call(DEVICE_UPLOAD, method.name, path, org.json.JSONArray(parts.map(::devicePartJson)))) }

internal suspend fun <T> deviceRead(path: String, parse: (JSONObject) -> T?): T? =
    withContext(Dispatchers.Default) { Qgc.call(DEVICE_READ, path)?.takeIf { it.optBoolean("ok") }?.optJSONObject("result")?.let(parse) }

internal suspend fun deviceQr(text: String): ByteArray? =
    withContext(Dispatchers.Default) {
        Qgc.call(DEVICE_QR, text)?.takeIf { it.optBoolean("ok") }?.optText("result")?.takeIf { it.isNotEmpty() }?.let { android.util.Base64.decode(it, android.util.Base64.DEFAULT) }
    }

internal suspend fun deviceReach(url: String): Boolean? =
    withContext(Dispatchers.Default) { Qgc.call(DEVICE_REACH, url)?.takeIf { it.optBoolean("ok") }?.optBoolean("result") }
