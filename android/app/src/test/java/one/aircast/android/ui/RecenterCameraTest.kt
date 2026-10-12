package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class RecenterCameraTest {
    private fun gimbal(pitch: Double?, yaw: Double?) = GimbalIndicatorState("", "", "", "", false, false, false, false, "", false, emptyList(), pitchDegrees = pitch, yawDegrees = yaw)

    @Test
    fun `recenter shows only once the camera points away from centre`() {
        assertNull(recenterTarget(syntheticAims = true, syntheticPitch = DEFAULT_TILT_DEG, syntheticPan = 0.0, gimbal = null))
        assertEquals(RecenterTarget.Synthetic, recenterTarget(syntheticAims = true, syntheticPitch = DEFAULT_TILT_DEG, syntheticPan = 40.0, gimbal = null))
        assertNull(recenterTarget(syntheticAims = false, syntheticPitch = null, syntheticPan = null, gimbal = gimbal(0.5, -1.0)))
        assertEquals(RecenterTarget.Gimbal, recenterTarget(syntheticAims = false, syntheticPitch = null, syntheticPan = null, gimbal = gimbal(-45.0, 0.0)))
    }

    @Test
    fun `an aimable synthetic view is recentred rather than the gimbal behind it`() {
        assertNull(recenterTarget(syntheticAims = true, syntheticPitch = DEFAULT_TILT_DEG, syntheticPan = 0.0, gimbal = gimbal(-45.0, 0.0)))
    }

    @Test
    fun `nothing to recenter without an aimable view or a gimbal`() {
        assertNull(recenterTarget(syntheticAims = false, syntheticPitch = -60.0, syntheticPan = 90.0, gimbal = null))
    }
}
