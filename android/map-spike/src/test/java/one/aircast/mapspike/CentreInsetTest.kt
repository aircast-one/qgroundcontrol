package one.aircast.mapspike

import org.junit.Assert.assertFalse
import org.junit.Test

class CentreInsetTest {
    @Test
    fun `the map only recentres once the vehicle leaves the middle of the screen`() {
        assertFalse(outsideCentreInset(500f, 500f, 1000f, 1000f, 0f, 0f))
        assertTrue(outsideCentreInset(100f, 500f, 1000f, 1000f, 0f, 0f), "near the left edge")
        assertTrue(outsideCentreInset(500f, 700f, 1000f, 1000f, 0f, 200f), "behind the bottom controls")
        assertFalse("an unmeasured map never asks to move", outsideCentreInset(10f, 10f, 0f, 0f, 0f, 0f))
    }

    private fun assertTrue(condition: Boolean, message: String) = org.junit.Assert.assertTrue(message, condition)

    @Test
    fun theVehicleSitsInTheMiddleOfTheClearArea() {
        org.junit.Assert.assertEquals("chrome 300 px on top and 100 px below puts the vehicle 100 px under the map centre", 100f, clearAreaLift(300f, 100f), 0f)
        org.junit.Assert.assertEquals(0f, clearAreaLift(0f, 0f), 0f)
    }
}
