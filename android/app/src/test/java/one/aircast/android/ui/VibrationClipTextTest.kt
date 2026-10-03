package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class VibrationClipTextTest {
    @Test
    fun `each clip count names its accelerometer, like VibrationPage`() {
        assertEquals("Accel 1: 0 · Accel 2: 3 · Accel 3: 0", clipText(listOf(0, 3, 0)))
    }
}
