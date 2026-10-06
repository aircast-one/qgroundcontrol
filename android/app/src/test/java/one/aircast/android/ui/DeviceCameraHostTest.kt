package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class DeviceCameraHostTest {
    @Test
    fun `a device camera is turned upright for every way the screen can face`() {
        assertEquals(90, deviceCameraRotation(90, front = false, displayDegrees = 0))
        assertEquals(0, deviceCameraRotation(90, front = false, displayDegrees = 90))
        assertEquals(180, deviceCameraRotation(90, front = false, displayDegrees = 270))
        assertEquals(270, deviceCameraRotation(270, front = true, displayDegrees = 0))
        assertEquals(0, deviceCameraRotation(270, front = true, displayDegrees = 90))
        assertEquals(180, deviceCameraRotation(270, front = true, displayDegrees = 270))
    }
}
