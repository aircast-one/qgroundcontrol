package one.aircast.android.ui

import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import org.json.JSONObject
import one.aircast.mapspike.optText

internal fun hostNoticesPath(acknowledgedThrough: Long): String = "view.hostNotices($acknowledgedThrough)"

internal data class NoticeBatch(
    val through: Long,
    val destination: String?,
    val banners: List<String>,
    val unknownKinds: List<String>,
    val errorBanners: List<String>,
    val dialogs: List<AppMessage> = emptyList(),
)

internal data class AppMessage(val title: String, val text: String)

internal fun noticeBatch(view: JSONObject?): NoticeBatch? {
    val unseen = view?.optJSONArray("unseen") ?: return null
    val notices = (0 until unseen.length()).mapNotNull { unseen.optJSONObject(it) }.filter { it.optLong("id", -1L) >= 0 }
    if (notices.isEmpty()) return null
    val banners = view.optJSONArray("banners")
    return NoticeBatch(
        through = notices.maxOf { it.optLong("id") },
        destination = view.optText("destination").ifBlank { null },
        banners = (0 until (banners?.length() ?: 0)).mapNotNull { banners?.optString(it)?.ifBlank { null } },
        unknownKinds = notices.filter { !it.optBoolean("known", true) }.map { it.optText("kind") },
        errorBanners = notices.filter { it.optText("kind") == VEHICLE_ERROR_KIND }.map { it.optText("banner") }.filter { it.isNotBlank() }.distinct(),
        dialogs = view.optJSONArray("dialogs")?.let { listed -> (0 until listed.length()).mapNotNull { listed.optJSONObject(it) }.map { AppMessage(it.optText("title"), it.optText("text")) } }.orEmpty(),
    )
}

const val REPEAT_QUIET_MS = 30_000L
internal const val VEHICLE_ERROR_KIND = "vehicleError"
internal const val RESET_ERROR_LEVEL_MESSAGES = "vehicle.resetErrorLevelMessages"

internal const val ADDITIONAL_ERRORS = "Additional errors received"

internal fun criticalBanner(errors: List<String>): String? =
    errors.firstOrNull()?.let { first -> if (errors.size > 1) "$first \u00b7 $ADDITIONAL_ERRORS" else first }

@Composable
internal fun AppMessageDialog(message: AppMessage, onDismiss: () -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(message.title) },
        text = { Text(message.text) },
        confirmButton = { TextButton(onClick = onDismiss) { Text("OK") } },
    )
}

internal fun quietBanners(banners: List<String>, shownAt: Map<String, Long>, now: Long): List<String> =
    banners.distinct().filter { banner -> shownAt[banner]?.let { now - it < REPEAT_QUIET_MS } != true }
