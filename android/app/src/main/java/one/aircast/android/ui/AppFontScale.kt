package one.aircast.android.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Density
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc

private const val APP_FONT_POINT_SIZE = "settings.appSettings.appFontPointSize"
private const val PLATFORM_FONT_POINT_SIZE = 14f
private const val FONT_POINT_SIZE_MIN = 6f
private const val FONT_POINT_SIZE_MAX = 48f
private const val FONT_SIZE_POLL_MS = 1000L

internal fun appFontScale(pointSize: Any?): Float =
    (pointSize as? Number)?.toFloat()
        ?.takeIf { it in FONT_POINT_SIZE_MIN..FONT_POINT_SIZE_MAX }
        ?.div(PLATFORM_FONT_POINT_SIZE)
        ?: 1f

@Composable
fun AppFontScale(content: @Composable () -> Unit) {
    var scale by remember { mutableFloatStateOf(1f) }

    LaunchedEffect(Unit) {
        while (isActive) {
            scale = withContext(Dispatchers.Default) { appFontScale(Qgc.get(APP_FONT_POINT_SIZE)?.opt("value")) }
            delay(FONT_SIZE_POLL_MS)
        }
    }

    val density = LocalDensity.current
    CompositionLocalProvider(LocalDensity provides Density(density.density, density.fontScale * scale), content = content)
}
