package one.aircast.android.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import org.json.JSONObject
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainInOrder
import one.aircast.android.bridge.qgcPath

internal const val ADVANCED_UI_PATH = "view.advancedUi"
internal const val SET_ADVANCED_UI = "advancedUi.set"

internal fun advancedUiShown(view: JSONObject?): Boolean = view?.optBoolean("shown", true) ?: true

@Composable
internal fun advancedUiShown(): Boolean {
    val view by qgcPath(ADVANCED_UI_PATH)
    return advancedUiShown(view)
}

internal fun toggleAdvancedUi(shown: Boolean) = offMainInOrder { Qgc.invoke(SET_ADVANCED_UI, !shown) }
