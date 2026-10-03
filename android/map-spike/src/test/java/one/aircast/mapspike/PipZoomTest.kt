package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PipZoomTest {
    @Test
    fun `the small map zooms out by three and the main map returns to its own zoom like FlyViewMap`() {
        assertEquals(12.0, pipZoom(15.0, pip = true)!!, 0.0)
        assertNull("QGC leaves zoom 3 and below alone", pipZoom(2.5, pip = true))
        assertEquals(15.0, pipZoom(15.0, pip = false)!!, 0.0)
        assertNull(pipZoom(0.0, pip = false))
    }
}
