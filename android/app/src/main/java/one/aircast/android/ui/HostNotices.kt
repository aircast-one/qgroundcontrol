package one.aircast.android.ui

import org.json.JSONObject
import one.aircast.mapspike.optText

// view.hostNotices applies the notice rules - which notices come after the id this head has
// acknowledged, where the last navigation notice sends the operator, and one banner per distinct
// title and text for every kind but navigation. What stays here is the repeat window, because it is
// about what this head last drew.
internal fun hostNoticesPath(acknowledgedThrough: Long): String = "view.hostNotices($acknowledgedThrough)"

internal data class NoticeBatch(
    val through: Long,
    val destination: String?,
    val banners: List<String>,
    val unknownKinds: List<String>,
)

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
    )
}

const val REPEAT_QUIET_MS = 30_000L

internal fun quietBanners(banners: List<String>, shownAt: Map<String, Long>, now: Long): List<String> =
    banners.distinct().filter { banner -> shownAt[banner]?.let { now - it < REPEAT_QUIET_MS } != true }
