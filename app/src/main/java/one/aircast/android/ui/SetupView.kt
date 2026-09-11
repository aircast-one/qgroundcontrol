package one.aircast.android.ui

import org.json.JSONObject
import one.aircast.mapspike.optText

internal const val SETUP = "view.setup"

internal data class SetupPage(
    val name: String,
    val parameterSections: Boolean,
)

internal data class SetupGroup(val title: String, val pages: List<SetupPage>)

internal fun setupGroups(view: JSONObject?): List<SetupGroup> {
    val groups = view?.optJSONArray("groups") ?: return emptyList()
    return (0 until groups.length()).mapNotNull { index ->
        groups.optJSONObject(index)?.let { group ->
            val pages = group.optJSONArray("pages")
            SetupGroup(
                title = group.optText("title"),
                pages = (0 until (pages?.length() ?: 0)).mapNotNull { page ->
                    pages!!.optJSONObject(page)?.let {
                        SetupPage(
                            name = it.optText("name"),
                            parameterSections = it.optBoolean("parameterSections"),
                        )
                    }
                },
            )
        }
    }
}

internal data class SetupReadiness(val ready: Boolean, val headline: String, val detail: String)

internal fun setupReadiness(view: JSONObject?): SetupReadiness? = view?.let {
    SetupReadiness(
        ready = it.optBoolean("ready"),
        headline = it.optText("headline"),
        detail = it.optText("detail"),
    )
}

internal fun setupPage(view: JSONObject?, name: String): SetupPage? =
    setupGroups(view).flatMap { it.pages }.firstOrNull { it.name == name }

internal fun setupPagePath(name: String): String = "$SETUP($name)"

private val WATCHES_BUT_CANNOT_FINISH = setOf(RADIO)

internal fun headFinishes(page: String): Boolean = page !in WATCHES_BUT_CANNOT_FINISH

internal fun setupBadge(page: String, openable: Boolean): String =
    if (openable && !headFinishes(page)) "Finish on desktop" else "Needs setup"
