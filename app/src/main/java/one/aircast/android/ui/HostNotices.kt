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

internal fun noticeToShow(notices: List<HostNotice>): HostNotice? =
    notices.firstOrNull { it.kind == NOTICE_VEHICLE_ERROR }
        ?: notices.firstOrNull { it.kind == NOTICE_MESSAGE }

internal fun noticeBanner(notice: HostNotice): String =
    listOf(notice.title, notice.text).filter { it.isNotBlank() }.joinToString(" · ")
