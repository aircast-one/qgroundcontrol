package one.aircast.mapspike

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp

private val Cyan = Color(0xFF4DD0E1)
private val CyanDeep = Color(0xFF00363D)
private val CyanInk = Color(0xFF003A41)
private val Amber = Color(0xFFFFD54F)
private val AmberInk = Color(0xFF3E2E00)
private val Red = Color(0xFFFF5252)
private val RedInk = Color(0xFF410002)
private val Slate = Color(0xFFB0BEC5)
private val SlateDeep = Color(0xFF263238)

private val Ink = Color(0xFF0E1113)
private val InkRaised = Color(0xFF161A1D)
private val Paper = Color(0xFFF7F9FA)
private val PaperRaised = Color(0xFFFFFFFF)
private val OnInk = Color(0xFFE6EAED)
private val OnPaper = Color(0xFF11181C)

private val AircastDark = darkColorScheme(
    primary = Cyan,
    onPrimary = CyanInk,
    primaryContainer = CyanDeep,
    onPrimaryContainer = Cyan,
    secondary = Slate,
    onSecondary = SlateDeep,
    secondaryContainer = SlateDeep,
    onSecondaryContainer = Slate,
    tertiary = Amber,
    onTertiary = AmberInk,
    tertiaryContainer = AmberInk,
    onTertiaryContainer = Amber,
    error = Red,
    onError = RedInk,
    errorContainer = Color(0xFF5C0F12),
    onErrorContainer = Red,
    background = Ink,
    onBackground = OnInk,
    surface = Ink,
    onSurface = OnInk,
    surfaceVariant = InkRaised,
    onSurfaceVariant = Color(0xFFBEC7CC),
    outline = Color(0xFF7A868C),
    outlineVariant = Color(0xFF3A4247),
    surfaceDim = Ink,
    surfaceBright = Color(0xFF2A3034),
    surfaceContainerLowest = Color(0xFF070A0B),
    surfaceContainerLow = Color(0xFF13171A),
    surfaceContainer = InkRaised,
    surfaceContainerHigh = Color(0xFF1E2427),
    surfaceContainerHighest = Color(0xFF262D31),
    inverseSurface = OnInk,
    inverseOnSurface = Ink,
    scrim = Color(0xFF000000),
)

private val AircastLight = lightColorScheme(
    primary = Color(0xFF006874),
    onPrimary = Color(0xFFFFFFFF),
    primaryContainer = Color(0xFF9EEFFD),
    onPrimaryContainer = Color(0xFF001F24),
    secondary = Color(0xFF37474F),
    onSecondary = Color(0xFFFFFFFF),
    secondaryContainer = Color(0xFFCFD8DC),
    onSecondaryContainer = Color(0xFF11181C),
    tertiary = Color(0xFF7A5900),
    onTertiary = Color(0xFFFFFFFF),
    tertiaryContainer = Color(0xFFFFDF9B),
    onTertiaryContainer = Color(0xFF261A00),
    error = Color(0xFFBA1A1A),
    onError = Color(0xFFFFFFFF),
    errorContainer = Color(0xFFFFDAD6),
    onErrorContainer = Color(0xFF410002),
    background = Paper,
    onBackground = OnPaper,
    surface = PaperRaised,
    onSurface = OnPaper,
    surfaceVariant = Color(0xFFDCE4E8),
    onSurfaceVariant = Color(0xFF3F484C),
    outline = Color(0xFF6F797E),
    outlineVariant = Color(0xFFBFC8CC),
    surfaceDim = Color(0xFFD8DEE1),
    surfaceBright = PaperRaised,
    surfaceContainerLowest = PaperRaised,
    surfaceContainerLow = Color(0xFFF2F5F7),
    surfaceContainer = Color(0xFFECF0F2),
    surfaceContainerHigh = Color(0xFFE6EBEE),
    surfaceContainerHighest = Color(0xFFE0E6E9),
    inverseSurface = Color(0xFF2B3134),
    inverseOnSurface = Color(0xFFEFF3F5),
    scrim = Color(0xFF000000),
)

// Monospace rather than a "tnum" feature setting: the platform default font does not
// carry tabular figures on every device, and a reading whose digits change width makes
// the whole row shuffle as the number moves.
val TelemetryNumber = TextStyle(
    fontFamily = FontFamily.Monospace,
    fontWeight = FontWeight.Medium,
    fontSize = 19.sp,
    lineHeight = 24.sp,
)

private val AircastTypography = Typography()

@Composable
fun AircastTheme(dark: Boolean = isSystemInDarkTheme(), content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = if (dark) AircastDark else AircastLight,
        typography = AircastTypography,
        content = content,
    )
}
