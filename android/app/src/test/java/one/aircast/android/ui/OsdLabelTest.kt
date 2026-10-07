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
