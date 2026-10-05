package one.aircast.android.ui

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import one.aircast.map.FlyMapEdits

internal class FlyScreenState(val layout: OverlayLayoutState) {
    var refusal by mutableStateOf<String?>(null)
    var deckRequest by mutableStateOf<String?>(null)
    var choosingReadings by mutableStateOf(false)
    var pendingMode by mutableStateOf<String?>(null)
    var instrumentEdits by mutableIntStateOf(0)
    var guidedPanels by mutableIntStateOf(0)
    val guidedPanelOpen: Boolean get() = guidedPanels > 0
    val controlRequestDeadlines = mutableStateMapOf<Int, Long>()
    val mapEdits = FlyMapEdits()
}

internal val LocalFlyScreenState = staticCompositionLocalOf<FlyScreenState> { error("FlyScreenState is provided by AircastShell") }
