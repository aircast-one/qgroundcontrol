package one.aircast.mapspike

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
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
    success = Color(0xFF7BDA8F),
    onSuccess = Color(0xFF00391A),
    successContainer = Color(0xFF005229),
    warning = Color(0xFFF2C06B),
    onWarning = Color(0xFF422C00),
    warningContainer = Color(0xFF5F4100),
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
    success = Color(0xFF1B6D36),
    onSuccess = Color(0xFFFFFFFF),
    successContainer = Color(0xFFA6F5B5),
    warning = Color(0xFF7C5800),
    onWarning = Color(0xFFFFFFFF),
    warningContainer = Color(0xFFFFDEA6),
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

internal val AircastDark = darkColorScheme(
    primary = Color(0xFFA9C7FF),
    onPrimary = Color(0xFF07305F),
    primaryContainer = Color(0xFF254777),
    onPrimaryContainer = Color(0xFFD6E3FF),
    secondary = Color(0xFFBCC7DC),
    onSecondary = Color(0xFF273141),
    secondaryContainer = Color(0xFF3D4758),
    onSecondaryContainer = Color(0xFFD8E3F8),
    tertiary = Color(0xFFDBBCE1),
    onTertiary = Color(0xFF3E2845),
    tertiaryContainer = Color(0xFF563E5C),
    error = Color(0xFFFFB4AB),
    onError = Color(0xFF690005),
    errorContainer = Color(0xFF93000A),
    onErrorContainer = Color(0xFFFFDAD6),
    background = Color(0xFF111318),
    onBackground = Color(0xFFE2E2E9),
    surface = Color(0xFF111318),
    onSurface = Color(0xFFE2E2E9),
    surfaceVariant = Color(0xFF44474E),
    onSurfaceVariant = Color(0xFFC4C6D0),
    outline = Color(0xFF8E9099),
    outlineVariant = Color(0xFF44474E),
    surfaceDim = Color(0xFF111318),
    surfaceBright = Color(0xFF37393E),
    surfaceContainerLowest = Color(0xFF0C0E13),
    surfaceContainerLow = Color(0xFF191C20),
    surfaceContainer = Color(0xFF1D2024),
    surfaceContainerHigh = Color(0xFF282A2F),
    surfaceContainerHighest = Color(0xFF33353A),
    inverseSurface = Color(0xFFE2E2E9),
    inverseOnSurface = Color(0xFF2E3036),
    scrim = Color(0xFF000000),
)

internal val AircastLight = lightColorScheme(
    primary = Color(0xFF3A5F93),
    onPrimary = Color(0xFFFFFFFF),
    primaryContainer = Color(0xFFD6E3FF),
    onPrimaryContainer = Color(0xFF001B3E),
    secondary = Color(0xFF555F71),
    onSecondary = Color(0xFFFFFFFF),
    secondaryContainer = Color(0xFFD8E3F8),
    onSecondaryContainer = Color(0xFF121C2B),
    tertiary = Color(0xFF6E5676),
    onTertiary = Color(0xFFFFFFFF),
    tertiaryContainer = Color(0xFFF7D8FF),
    error = Color(0xFFBA1A1A),
    onError = Color(0xFFFFFFFF),
    errorContainer = Color(0xFFFFDAD6),
    onErrorContainer = Color(0xFF410002),
    background = Color(0xFFF9F9FF),
    onBackground = Color(0xFF191C20),
    surface = Color(0xFFF9F9FF),
    onSurface = Color(0xFF191C20),
    surfaceVariant = Color(0xFFE2E2E9),
    onSurfaceVariant = Color(0xFF44474E),
    outline = Color(0xFF74777F),
    outlineVariant = Color(0xFFC4C6D0),
    surfaceDim = Color(0xFFD9D9E0),
    surfaceBright = Color(0xFFF9F9FF),
    surfaceContainerLowest = Color(0xFFFFFFFF),
    surfaceContainerLow = Color(0xFFF3F3FA),
    surfaceContainer = Color(0xFFEDEDF4),
    surfaceContainerHigh = Color(0xFFE7E8EE),
    surfaceContainerHighest = Color(0xFFE2E2E9),
    inverseSurface = Color(0xFF2E3036),
    inverseOnSurface = Color(0xFFF0F0F7),
    scrim = Color(0xFF000000),
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
