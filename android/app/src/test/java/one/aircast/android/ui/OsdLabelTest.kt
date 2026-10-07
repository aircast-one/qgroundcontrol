package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class OsdLabelTest {
    @Test
    fun `the flying readings shorten to DJI's letters and anything else keeps its name`() {
        assertEquals(listOf("D", "H", "H.S", "V.S"), listOf("Distance to home", "Alt (Rel)", "Ground Speed", "Climb Rate").map(::osdLabel))
        assertEquals("FLIGHT TIME", osdLabel("Flight time"))
    }
}

class DjiChromeTest {
    @Test
    fun `the status title splits into plain mode text and a note for the pill`() {
        assertEquals("Manual", osdModeText("Manual · Not fully ready"))
        assertEquals("Not fully ready", osdStatusNote("Manual · Not fully ready"))
        assertEquals(null, osdStatusNote("Manual"))
    }

    @Test
    fun `speeds go on the small top row and the rest on the large one`() {
        assertEquals(listOf(true, true, false, false), listOf("Ground Speed", "Climb Rate", "Alt (Rel)", "Distance to home").map(::osdIsSpeed))
    }

    @Test
    fun `the battery ring reads the percentage out of the cell text`() {
        assertEquals(53, batteryPercent("B1 53%"))
        assertEquals(null, batteryPercent("No battery"))
    }

    @Test
    fun `the mini-map opens as a thumbnail unless the pilot chose otherwise`() {
        assertEquals(MiniMap.Thumb, miniMapNamed(null))
        assertEquals(MiniMap.Compass, miniMapNamed("Compass"))
        assertEquals(MiniMap.Thumb, miniMapNamed("Unknown"))
    }
}
