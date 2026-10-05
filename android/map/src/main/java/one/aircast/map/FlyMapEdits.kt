package one.aircast.map

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf

class FlyMapEdits {
    var orbit by mutableStateOf<OrbitCircle?>(null)
    var gotoLoiter by mutableStateOf<LoiterEdit?>(null)
}

val LocalFlyMapEdits = staticCompositionLocalOf<FlyMapEdits> { error("FlyMapEdits is provided by the screen that hosts the map") }
