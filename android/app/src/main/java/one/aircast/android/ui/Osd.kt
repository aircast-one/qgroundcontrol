package one.aircast.android.ui

import android.graphics.BlendMode
import android.graphics.BlendModeColorFilter
import android.graphics.RenderEffect
import android.graphics.Shader
import android.os.Build
import androidx.annotation.RequiresApi
import androidx.compose.runtime.Composable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asComposeRenderEffect
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.unit.dp

internal val LocalFlyOsd = compositionLocalOf { false }

private val OSD_SHADOW_BLUR = 3.dp
private val OSD_SHADOW_DROP = 1.dp

@Composable
internal fun osdBackdrop(color: Color): Color = if (LocalFlyOsd.current) Color.Transparent else color

@Composable
internal fun osdTint(color: Color, osdColor: Color): Color = if (LocalFlyOsd.current) osdColor else color

internal fun Modifier.osdShadow(): Modifier =
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) graphicsLayer { renderEffect = osdShadowEffect(OSD_SHADOW_BLUR.toPx(), OSD_SHADOW_DROP.toPx()) } else this

@RequiresApi(Build.VERSION_CODES.S)
private fun osdShadowEffect(blur: Float, drop: Float): androidx.compose.ui.graphics.RenderEffect {
    val halo = RenderEffect.createOffsetEffect(
        0f,
        drop,
        RenderEffect.createBlurEffect(
            blur,
            blur,
            RenderEffect.createColorFilterEffect(BlendModeColorFilter(android.graphics.Color.BLACK, BlendMode.SRC_IN)),
            Shader.TileMode.DECAL,
        ),
    )
    val doubled = RenderEffect.createBlendModeEffect(halo, halo, BlendMode.SRC_OVER)
    return RenderEffect.createBlendModeEffect(doubled, RenderEffect.createOffsetEffect(0f, 0f), BlendMode.SRC_OVER).asComposeRenderEffect()
}
