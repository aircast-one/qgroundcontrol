package one.aircast.map

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.Typography
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

@Immutable
data class AircastColors(
    val success: Color,
    val onSuccess: Color,
    val successContainer: Color,
    val warning: Color,
    val onWarning: Color,
    val warningContainer: Color,
    val mission: Color,
    val alert: Color,
    val mapLand: Color,
    val mapWater: Color,
    val mapRoad: Color,
    val outdoorBackground: Color,
    val outdoorForeground: Color,
    val outdoorAccent: Color,
    val onOutdoorAccent: Color,
)

internal val AircastDarkColors = AircastColors(
    success = AircastTokens.Dark.success,
    onSuccess = AircastTokens.Dark.onSuccess,
    successContainer = AircastTokens.Dark.successContainer,
    warning = AircastTokens.Dark.warning,
    onWarning = AircastTokens.Dark.onWarning,
    warningContainer = AircastTokens.Dark.warningContainer,
    mission = Color(0xFFFFB870),
    alert = Color(0xFFFFB870),
    mapLand = Color(0xFF1B2A24),
    mapWater = Color(0xFF12263A),
    mapRoad = Color(0xFF3A4A44),
    outdoorBackground = Color(0xFF000000),
    outdoorForeground = Color(0xFFFFFFFF),
    outdoorAccent = Color(0xFFFFD60A),
    onOutdoorAccent = Color(0xFF000000),
)

internal val AircastLightColors = AircastColors(
    success = AircastTokens.Light.success,
    onSuccess = AircastTokens.Light.onSuccess,
    successContainer = AircastTokens.Light.successContainer,
    warning = AircastTokens.Light.warning,
    onWarning = AircastTokens.Light.onWarning,
    warningContainer = AircastTokens.Light.warningContainer,
    mission = Color(0xFFB45F00),
    alert = Color(0xFFB45F00),
    mapLand = Color(0xFFDDE8DC),
    mapWater = Color(0xFFB9D7F0),
    mapRoad = Color(0xFFFFFFFF),
    outdoorBackground = Color(0xFF000000),
    outdoorForeground = Color(0xFFFFFFFF),
    outdoorAccent = Color(0xFFFFD60A),
    onOutdoorAccent = Color(0xFF000000),
)

internal val AircastShapes = Shapes(
    extraSmall = RoundedCornerShape(4.dp),
    small = RoundedCornerShape(8.dp),
    medium = RoundedCornerShape(12.dp),
    large = RoundedCornerShape(16.dp),
    extraLarge = RoundedCornerShape(28.dp),
)

private fun TextStyle.unspaced() = copy(letterSpacing = 0.sp)

val AircastTypography = Typography().let {
    Typography(
        displayLarge = it.displayLarge.unspaced(),
        displayMedium = it.displayMedium.unspaced(),
        displaySmall = it.displaySmall.unspaced(),
        headlineLarge = it.headlineLarge.unspaced(),
        headlineMedium = it.headlineMedium.unspaced(),
        headlineSmall = it.headlineSmall.unspaced(),
        titleLarge = it.titleLarge.unspaced(),
        titleMedium = it.titleMedium.unspaced(),
        titleSmall = it.titleSmall.unspaced(),
        bodyLarge = it.bodyLarge.unspaced(),
        bodyMedium = it.bodyMedium.unspaced(),
        bodySmall = it.bodySmall.unspaced(),
        labelLarge = it.labelLarge.unspaced(),
        labelMedium = it.labelMedium.unspaced(),
        labelSmall = it.labelSmall.unspaced(),
    )
}

val TelemetryNumber = TextStyle(
    fontFamily = FontFamily.Default,
    fontWeight = FontWeight.Medium,
    fontSize = 22.sp,
    lineHeight = 26.4.sp,
    fontFeatureSettings = "tnum",
)

val TelemetryNumberMedium = TelemetryNumber.copy(fontSize = 16.sp, lineHeight = 20.sp)

object AircastSpace {
    val s1 = 4.dp
    val s2 = 8.dp
    val s3 = 12.dp
    val s4 = 16.dp
    val s5 = 20.dp
    val s6 = 24.dp
    val s8 = 32.dp
}

private val LocalAircastColors = staticCompositionLocalOf { AircastDarkColors }

val MaterialTheme.aircast: AircastColors
    @Composable
    @ReadOnlyComposable
    get() = LocalAircastColors.current

@Composable
fun AircastTheme(dark: Boolean = isSystemInDarkTheme(), content: @Composable () -> Unit) {
    CompositionLocalProvider(LocalAircastColors provides if (dark) AircastDarkColors else AircastLightColors) {
        MaterialTheme(
            colorScheme = if (dark) AircastDark else AircastLight,
            shapes = AircastShapes,
            typography = AircastTypography,
            content = content,
        )
    }
}
