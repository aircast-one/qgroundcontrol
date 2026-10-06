package one.aircast.android.ui

import androidx.compose.runtime.getValue
import androidx.compose.ui.geometry.Rect
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import one.aircast.map.FlyMapEdits

internal class FlyScreenState(val layout: OverlayLayoutState) {
    var refusal by mutableStateOf<String?>(null)
    var deckRequest by mutableStateOf<String?>(null)
    var requestedSheet by mutableStateOf<String?>(null)
    var choosingReadings by mutableStateOf(false)
    var pendingMode by mutableStateOf<String?>(null)
    var instrumentEdits by mutableIntStateOf(0)
    var guidedPanels by mutableIntStateOf(0)
    val guidedPanelOpen: Boolean get() = guidedPanels > 0
    val controlRequestDeadlines = mutableStateMapOf<Int, Long>()
    val mapEdits = FlyMapEdits()
    val obstacles = mutableStateMapOf<String, Rect>()
    var mapInsets by mutableStateOf(MapInsets(top = 0, bottom = 0))
}

internal val LocalFlyScreenState = staticCompositionLocalOf<FlyScreenState> { error("FlyScreenState is provided by AircastShell") }

internal val REQUESTABLE_SHEETS = setOf("more", "readings", "camera", "gimbal", "indicators", "status", "modes")

@androidx.compose.runtime.Composable
internal fun OpenOnRequest(name: String, open: () -> Unit) {
    val flyScreen = LocalFlyScreenState.current
    androidx.compose.runtime.LaunchedEffect(flyScreen.requestedSheet) {
        if (flyScreen.requestedSheet == name) {
            flyScreen.requestedSheet = null
            open()
        }
    }
}
