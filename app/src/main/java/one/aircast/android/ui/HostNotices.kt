package one.aircast.android.ui

import org.json.JSONObject

internal const val NOTICE_MESSAGE = "message"
internal const val NOTICE_VEHICLE_ERROR = "vehicleError"
internal const val NOTICE_NAVIGATION = "navigation"
internal val NOTICE_KINDS = setOf(NOTICE_MESSAGE, NOTICE_VEHICLE_ERROR, NOTICE_NAVIGATION)

internal data class HostNotice(
    val id: Long,
    val kind: String,
    val title: String,
    val text: String,
)

internal fun hostNotices(view: JSONObject?): List<HostNotice> {
    val items = view?.optJSONArray("notices") ?: return emptyList()
    return (0 until items.length()).mapNotNull { index ->
        items.optJSONObject(index)?.let {
            HostNotice(
                id = it.optLong("id", -1L),
                kind = it.optString("kind"),
                title = it.optString("title"),
                text = it.optString("text"),
            )
        }
    }.filter { it.id >= 0 }.onEach {
        if (it.kind !in NOTICE_KINDS) {
            android.util.Log.w("HostNotices", "unrecognised notice kind '" + it.kind + "' - showing it rather than guessing")
        }
    }
}

internal fun noticeDestination(notices: List<HostNotice>): String? =
    notices.lastOrNull { it.kind == NOTICE_NAVIGATION }?.title?.ifBlank { null }

internal fun noticesAfter(notices: List<HostNotice>, acknowledgedThrough: Long): List<HostNotice> =
    notices.filter { it.id > acknowledgedThrough }

internal fun noticesToShow(notices: List<HostNotice>): List<HostNotice> =
    notices.filter { it.kind != NOTICE_NAVIGATION }

internal fun noticeBanner(notice: HostNotice): String =
    listOf(notice.title, notice.text).filter { it.isNotBlank() }.joinToString(" · ")

internal fun bannersToShow(notices: List<HostNotice>, lastShown: String?): List<String> =
    noticesToShow(notices)
        .map { noticeBanner(it) }
        .fold(emptyList<String>()) { kept, banner ->
            when (banner) {
                kept.lastOrNull() ?: lastShown -> kept
                else -> kept + banner
            }
        }
