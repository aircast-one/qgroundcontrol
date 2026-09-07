package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class RemoteSupportScreenTest {
    @Test
    fun `a plain host or address is usable`() {
        assertEquals(true, supportHostIsUsable("support.ardupilot.org"))
        assertEquals(true, supportHostIsUsable("10.0.0.4:14550"))
    }

    @Test
    fun `a blank or spaced host is refused`() {
        assertEquals(false, supportHostIsUsable(""))
        assertEquals(false, supportHostIsUsable("   "))
        assertEquals(false, supportHostIsUsable("two hosts"))
    }

    @Test
    fun `remote support is openable on both firmwares`() {
        assertEquals(true, hasNativeSetupPage(REMOTE_SUPPORT, isPx4 = false))
        assertEquals(true, hasNativeSetupPage(REMOTE_SUPPORT, isPx4 = true))
    }

    @Test
    fun `a component with neither a form nor a custom page stays closed`() {
        assertEquals(false, hasNativeSetupPage("Motors", isPx4 = false))
        assertEquals(false, hasNativeSetupPage("Motors", isPx4 = true))
        assertEquals(false, hasNativeSetupPage("Radio", isPx4 = false))
    }

    @Test
    fun `sensor calibration opens for arducopter only`() {
        assertEquals(true, hasNativeSetupPage(SENSORS, isPx4 = false))
        assertEquals(false, hasNativeSetupPage(SENSORS, isPx4 = true))
    }

    @Test
    fun `no calibration offered here spins a motor`() {
        assertEquals(
            emptyList<String>(),
            CALIBRATIONS.map { it.method }.filter { it == "calibrateMotorInterference" },
        )
    }
}
