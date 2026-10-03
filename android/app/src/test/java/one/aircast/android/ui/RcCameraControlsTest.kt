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

    @Test
    fun `record shows on when either the stream or the record channel records, as _recording does`() {
        assertTrue(cameraRecording(recordChannel = 0, channelRecording = false, streamRecording = true))
        assertTrue(cameraRecording(recordChannel = 5, channelRecording = true, streamRecording = false))
        assertFalse(cameraRecording(recordChannel = 0, channelRecording = true, streamRecording = false))
    }
}

class VehicleAltitudeTest {

    @Test
    fun `the vehicle altitude copied is the one in the item's frame, as EditPositionDialog picks it`() {
        assertEquals("vehicle.altitudeRelative", vehicleAltitudePath(1))
        assertEquals("vehicle.altitudeAMSL", vehicleAltitudePath(2))
        assertEquals("vehicle.altitudeAboveTerr", vehicleAltitudePath(3))
        assertEquals("vehicle.altitudeAboveTerr", vehicleAltitudePath(4))
        assertEquals(null, vehicleAltitudePath(null))
    }
}
