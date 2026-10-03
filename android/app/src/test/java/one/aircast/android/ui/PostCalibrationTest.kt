package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class PostCalibrationTest {
    @Test
    fun `only a completed ardupilot accel or compass calibration asks for a reboot`() {
        assertEquals("YOU MUST REBOOT YOUR VEHICLE AFTER EACH CALIBRATION.", postCalibrationPrompt(ACCEL_ROUTINE, CALIBRATION_COMPLETE, px4 = false))
        assertTrue(postCalibrationPrompt(COMPASS_ROUTINE, CALIBRATION_COMPLETE, px4 = false)!!.startsWith("Shown in the indicator bars"))
        assertNull("a cancelled or failed run offers nothing", postCalibrationPrompt(COMPASS_ROUTINE, "Calibration cancelled", px4 = false))
        assertNull(postCalibrationPrompt("gyro", CALIBRATION_COMPLETE, px4 = false))
        assertNull(postCalibrationPrompt(ACCEL_ROUTINE, CALIBRATION_COMPLETE, px4 = true))
        assertEquals("Reboot the vehicle prior to flight.", postCalibrationPrompt(COMPASS_ROUTINE, CALIBRATION_COMPLETE, px4 = true))
        assertEquals("Compass calibration complete", postCalibrationTitle(COMPASS_ROUTINE, px4 = true))
        assertEquals(CALIBRATION_COMPLETE, postCalibrationTitle(COMPASS_ROUTINE, px4 = false))
    }
}
