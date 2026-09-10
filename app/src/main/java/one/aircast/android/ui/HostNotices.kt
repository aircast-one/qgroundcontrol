package one.aircast.android.ui

import org.json.JSONObject

internal const val NOTICE_MESSAGE = 0
internal const val NOTICE_VEHICLE_ERROR = 1
internal const val NOTICE_NAVIGATION = 2

internal data class HostNotice(
    val id: Long,
    val kind: Int,
    val title: String,
    val text: String,
)

internal fun hostNotices(view: JSONObject?): List<HostNotice> {
    val items = view?.optJSONArray("notices") ?: return emptyList()
    return (0 until items.length()).mapNotNull { index ->
        items.optJSONObject(index)?.let {
            HostNotice(
                id = it.optLong("id", -1L),
                kind = it.optInt("kind", NOTICE_MESSAGE),
                title = it.optString("title"),
                text = it.optString("text"),
            )
        }
    }.filter { it.id >= 0 }
}

internal fun noticeDestination(notices: List<HostNotice>): String? =
    notices.lastOrNull { it.kind == NOTICE_NAVIGATION }?.title?.ifBlank { null }

// Every notice, in the order it happened. Showing one and acknowledging the batch is the
// silent drop the queue exists to prevent, and arrival order keeps a pair together - the
// navigation that moved the operator and the message saying why.
internal fun noticesToShow(notices: List<HostNotice>): List<HostNotice> =
    notices.filter { it.kind != NOTICE_NAVIGATION }

internal fun noticeBanner(notice: HostNotice): String =
    listOf(notice.title, notice.text).filter { it.isNotBlank() }.joinToString(" · ")
