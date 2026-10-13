package one.aircast.map

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class PipZoomTest {
    @Test
    fun `the small map zooms out by three and the main map returns to its own zoom like FlyViewMap`() {
        assertEquals(12.0, pipZoom(15.0, pip = true)!!, 0.0)
        assertNull("QGC leaves zoom 3 and below alone", pipZoom(2.5, pip = true))
        assertEquals(15.0, pipZoom(15.0, pip = false)!!, 0.0)
        assertNull(pipZoom(0.0, pip = false))
    }

    @Test
    fun `the zoomed out small map level is never kept as the main map's zoom`() {
        assertTrue(keepsZoom(15.0, pip = false, zoomedOutForPip = false))
        assertFalse("the small map's own zoom is not the pilot's", keepsZoom(12.0, pip = true, zoomedOutForPip = true))
        assertFalse("growing back to full size reports the small map's zoom before it zooms in; keeping it lost three levels a swap until the map showed a continent", keepsZoom(12.0, pip = false, zoomedOutForPip = true))
        assertFalse(keepsZoom(0.5, pip = false, zoomedOutForPip = false))
    }
}
