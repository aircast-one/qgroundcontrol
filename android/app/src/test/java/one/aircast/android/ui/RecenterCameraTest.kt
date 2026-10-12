package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class RecenterCameraTest {
    private fun gimbal(pitch: Double?, yaw: Double?) = GimbalIndicatorState("", "", "", "", false, false, false, false, "", false, emptyList(), pitchDegrees = pitch, yawDegrees = yaw)

    @Test
    fun `recenter shows only once the camera points away from centre`() {
        assertNull(recenterTarget(syntheticAims = true, syntheticTilt = DEFAULT_TILT_DEG, syntheticPan = 0.0, gimbal = null))
        assertEquals(RecenterTarget.Synthetic, recenterTarget(syntheticAims = true, syntheticTilt = DEFAULT_TILT_DEG, syntheticPan = 40.0, gimbal = null))
        assertNull(recenterTarget(syntheticAims = false, syntheticTilt = null, syntheticPan = null, gimbal = gimbal(0.5, -1.0)))
        assertEquals(RecenterTarget.Gimbal, recenterTarget(syntheticAims = false, syntheticTilt = null, syntheticPan = null, gimbal = gimbal(-45.0, 0.0)))
    }

    @Test
    fun `an aimable synthetic view is recentred rather than the gimbal behind it`() {
        assertNull(recenterTarget(syntheticAims = true, syntheticTilt = DEFAULT_TILT_DEG, syntheticPan = 0.0, gimbal = gimbal(-45.0, 0.0)))
    }

    @Test
    fun `the readout rounds to whole degrees`() {
        assertEquals("-11\u00b0", tiltText(-11.4))
        assertEquals("0\u00b0", tiltText(0.2))
    }

    @Test
    fun `nothing to recenter without an aimable view or a gimbal`() {
        assertNull(recenterTarget(syntheticAims = false, syntheticTilt = -60.0, syntheticPan = 90.0, gimbal = null))
    }
}
