package one.aircast.android.ui

import one.aircast.android.bridge.FactSlider
import org.junit.Assert.assertEquals
import org.junit.Test

class FactSliderStepsTest {
    @Test
    fun `a stepped slider snaps to its step, a fine or unstepped one stays continuous`() {
        assertEquals(9, sliderSteps(FactSlider(0f, 10f, 1f, 0, "")))
        assertEquals("10 km in 1 m steps is too fine to snap, the typed field covers precision", 0, sliderSteps(FactSlider(0f, 10000f, 1f, 0, "")))
        assertEquals(0, sliderSteps(FactSlider(0f, 10f, null, 2, "")))
    }
}
