package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class RcCameraControlsTest {

    @Test
    fun `a pan channel shared with tilt is dropped, as CameraControlLayer reads it`() {
        assertEquals(0, rcCameraChannels(tilt = 7, pan = 7, zoom = 0, light = 0, record = 0).pan)
        assertEquals(8, rcCameraChannels(tilt = 7, pan = 8, zoom = 0, light = 0, record = 0).pan)
        assertFalse(rcCameraChannels(0, 0, 0, 0, 0).any)
        assertTrue(rcCameraChannels(0, 0, 9, 0, 0).any)
    }
}
