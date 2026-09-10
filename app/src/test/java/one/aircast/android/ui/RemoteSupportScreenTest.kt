package one.aircast.android.ui

import org.junit.Assert.assertEquals
import one.aircast.android.bridge.Fact
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
    fun `pwm maps onto the bar and clamps outside the rc range`() {
        assertEquals(0f, pwmFraction(1000))
        assertEquals(0.5f, pwmFraction(1500))
        assertEquals(1f, pwmFraction(2000))
        assertEquals(0f, pwmFraction(800))
        assertEquals(1f, pwmFraction(2400))
    }

    @Test
    fun `no calibration offered here spins a motor`() {
        assertEquals(
            emptyList<String>(),
            CALIBRATIONS.map { it.method }.filter { it == "calibrateMotorInterference" },
        )
    }

    @Test
    fun `a value outside the listed options is not treated as an enum`() {
        val listed = Fact(
            path = "p", name = "ACRO_TRAINER", description = "", units = "",
            valueString = "2", value = 2, enumStrings = listOf("Disabled", "Leveling", "Leveling and Limited"),
            enumIndex = 2, isBool = false, isString = false, readOnly = false,
        )
        val offList = listed.copy(
            name = "ACRO_RP_RATE_TC",
            valueString = "Unknown: 0",
            enumStrings = listOf("Disabled", "Leveling", "Leveling and Limited", "Unknown: 0"),
            enumIndex = 3,
        )
        assertEquals(false, listed.valueIsOffTheEnumList)
        assertEquals(true, offList.valueIsOffTheEnumList)
    }
}
