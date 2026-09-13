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
            unknownEnumLabel = "Unknown: 0",
        )
        assertEquals(false, listed.valueIsOffTheEnumList)
        assertEquals(true, offList.valueIsOffTheEnumList)
    }

    @Test
    fun `the synthetic entry is spotted in a language that is not english`() {
        val german = Fact(
            path = "p", name = "ACRO_RP_RATE_TC", description = "", units = "",
            valueString = "0", value = 0,
            enumStrings = listOf("Deaktiviert", "Nivellierung", "Unbekannt: 0"),
            enumIndex = 2, unknownEnumLabel = "Unbekannt: 0",
            isBool = false, isString = false, readOnly = false,
        )
        assertEquals(true, german.valueIsOffTheEnumList)
    }
}
