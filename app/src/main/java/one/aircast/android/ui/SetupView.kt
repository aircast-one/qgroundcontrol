package one.aircast.android.ui

import org.json.JSONObject

internal const val SETUP = "view.setup"

internal data class SetupPage(
    val name: String,
    val native: Boolean,
    val parameterSections: Boolean,
)

internal data class SetupGroup(val title: String, val pages: List<SetupPage>)

internal fun setupGroups(view: JSONObject?): List<SetupGroup> {
    val groups = view?.optJSONArray("groups") ?: return emptyList()
    return (0 until groups.length()).mapNotNull { index ->
        groups.optJSONObject(index)?.let { group ->
            val pages = group.optJSONArray("pages")
            SetupGroup(
                title = group.optString("title"),
                pages = (0 until (pages?.length() ?: 0)).mapNotNull { page ->
                    pages!!.optJSONObject(page)?.let {
                        SetupPage(
                            name = it.optString("name"),
                            native = it.optBoolean("native"),
                            parameterSections = it.optBoolean("parameterSections"),
                        )
                    }
                },
            )
        }
    }
}

internal fun setupPage(view: JSONObject?, name: String): SetupPage? =
    setupGroups(view).flatMap { it.pages }.firstOrNull { it.name == name }

internal fun setupPagePath(name: String): String = "$SETUP($name)"
