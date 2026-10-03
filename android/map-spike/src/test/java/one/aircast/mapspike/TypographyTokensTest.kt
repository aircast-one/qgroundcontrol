package one.aircast.mapspike

import androidx.compose.ui.unit.sp
import org.junit.Assert.assertEquals
import org.junit.Test

class TypographyTokensTest {
    @Test
    fun `every text style carries the Penpot zero letter spacing and its size`() {
        val styles = with(AircastTypography) {
            listOf(
                displayLarge, displayMedium, displaySmall, headlineLarge, headlineMedium, headlineSmall,
                titleLarge, titleMedium, titleSmall, bodyLarge, bodyMedium, bodySmall, labelLarge, labelMedium, labelSmall,
            )
        }
        assertEquals(List(styles.size) { 0.sp }, styles.map { it.letterSpacing })
        assertEquals(
            listOf(24.sp, 22.sp, 16.sp, 14.sp, 12.sp, 11.sp),
            with(AircastTypography) { listOf(headlineSmall, titleLarge, bodyLarge, bodyMedium, labelMedium, labelSmall).map { it.fontSize } },
        )
    }
}
