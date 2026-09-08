package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class VibrationScreenTest {
    @Test
    fun `verdict follows the published thresholds`() {
        assertEquals("OK", verdictFor(0.0))
        assertEquals("OK", verdictFor(VIBE_WARN - 0.1))
        assertEquals("Caution", verdictFor(VIBE_WARN))
        assertEquals("Caution", verdictFor(VIBE_HIGH - 0.1))
        assertEquals("High", verdictFor(VIBE_HIGH))
        assertEquals("High", verdictFor(VIBE_MAX * 2))
    }

    @Test
    fun `verdict says nothing without a reading`() {
        assertEquals("", verdictFor(Double.NaN))
    }

    @Test
    fun `bar fraction spans zero to full across the scale`() {
        assertEquals(0f, barFraction(0.0), 1e-6f)
        assertEquals(0.5f, barFraction(VIBE_MAX / 2), 1e-6f)
        assertEquals(1f, barFraction(VIBE_MAX), 1e-6f)
    }

    @Test
    fun `bar fraction clamps beyond the scale and on bad input`() {
        assertEquals(1f, barFraction(VIBE_MAX * 10), 1e-6f)
        assertEquals(0f, barFraction(-5.0), 1e-6f)
        assertEquals(0f, barFraction(Double.NaN), 1e-6f)
    }
}

class VibrationMatchesQtBuildTest {
    @Test
    fun `the scale and thresholds are the ones VibrationPage qml uses`() {
        assertEquals(90.0, VIBE_MAX, 0.0)
        assertEquals(30.0, VIBE_WARN, 0.0)
        assertEquals(60.0, VIBE_HIGH, 0.0)
    }

    @Test
    fun `bar height follows the Qt build's formula`() {
        val qtBarFraction = { v: Double -> (minOf(90.0, v) / 90.0) }

        listOf(0.0, 15.0, 30.0, 60.0, 89.9, 90.0, 120.0).forEach { value ->
            assertEquals(qtBarFraction(value).toFloat(), barFraction(value), 1e-6f)
        }
    }

    @Test
    fun `a missing reading draws nothing rather than a full bar`() {
        assertEquals(0f, barFraction(Double.NaN), 0f)
    }
}
