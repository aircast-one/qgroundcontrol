package one.aircast.android.ui

import org.json.JSONObject
import one.aircast.mapspike.optText

internal const val SETUP = "view.setup"
internal const val FIRMWARE = "view.firmware"

// view.setup composes connected and the parameter load state from one read, so a disconnect cannot
// land between them the way two raw reads allowed.
internal fun parametersReady(view: JSONObject?): Boolean =
    view?.optBoolean("connected") == true && view.optBoolean("parametersReady")

internal data class SetupPage(
    val name: String,
    val parameterSections: Boolean,
    val screen: String = "",
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
                            screen = it.optText("screen"),
                        )
                    }
                },
            )
        }
    }
}

internal data class SetupReadiness(
    val ready: Boolean?,
    val setupComplete: Boolean? = null,
    val headline: String,
    val detail: String,
    val connected: Boolean,
    val firmware: String,
)

internal const val NO_VEHICLE_HEADLINE = "No Vehicle Connected"
internal const val NO_VEHICLE_TEXT = "Connect a vehicle to see and change its settings."
internal const val NOTHING_TO_CONFIGURE = "Nothing to Configure"
internal const val NOTHING_TO_CONFIGURE_TEXT = "Aircast doesn't support setup for this vehicle type. If it is already configured, you can still fly."

internal fun setupPillText(complete: Boolean): String = if (complete) "Setup complete" else "Needs setup"

internal fun setupReadiness(view: JSONObject?): SetupReadiness? = view?.let {
    SetupReadiness(
        ready = if (it.isNull("ready")) null else it.optBoolean("ready"),
        setupComplete = if (it.isNull("setupComplete") || !it.has("setupComplete")) null else it.optBoolean("setupComplete"),
        headline = it.optText("headline"),
        detail = it.optText("detail"),
        connected = it.optBoolean("connected"),
        firmware = it.optText("firmware"),
    )
}

internal fun isPx4(readiness: SetupReadiness?): Boolean = readiness?.firmware == "px4"

internal fun setupPage(view: JSONObject?, name: String): SetupPage? =
    setupGroups(view).flatMap { it.pages }.firstOrNull { it.name == name }

internal fun setupPagePath(name: String): String = "$SETUP($name)"

