package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ParametersScreenTest {
    @Test
    fun `a parameter path is addressable through the bridge`() {
        assertEquals(
            "vehicle.parameterManager.getParameter(-1,ACRO_BAL_PITCH)",
            parameterPath("ACRO_BAL_PITCH"),
        )
    }

    @Test
    fun `the path a row writes to is the one the bridge can resolve`() {
        val path = parameterPath("RTL_ALT")
        assertTrue("a bare name does not resolve", path.contains("."))
        assertTrue(path.startsWith("vehicle.parameterManager."))
        assertTrue(path.endsWith("(-1,RTL_ALT)"))
    }

    @Test
    fun `a parameter is found by what it does, not only by its name`() {
        assertTrue(parameterMatches("BATT_FS_LOW_ACT", "Low battery failsafe action", "failsafe"))
        assertTrue(parameterMatches("BATT_FS_LOW_ACT", "Low battery failsafe action", "BATT"))
        assertFalse(parameterMatches("BATT_FS_LOW_ACT", "Low battery failsafe action", "compass"))
    }

    @Test
    fun `searching is case insensitive on both halves`() {
        assertTrue(parameterMatches("RTL_ALT", "Return to launch altitude", "rtl_alt"))
        assertTrue(parameterMatches("RTL_ALT", "Return to launch altitude", "LAUNCH"))
    }

    @Test
    fun `a parameter whose description has not loaded yet still matches by name`() {
        assertTrue(parameterMatches("RTL_ALT", "", "RTL"))
        assertFalse(parameterMatches("RTL_ALT", "", "launch"))
    }

    @Test
    fun `an empty search keeps every parameter`() {
        assertTrue(parameterMatches("ANY", "", ""))
    }
}

class ParameterSubtitleTest {

    @org.junit.Test
    fun `a parameter says what it is and what it is measured in`() {
        org.junit.Assert.assertEquals(
            "RTL Altitude · cm",
            parameterSubtitle("RTL Altitude", "cm"),
        )
    }

    @org.junit.Test
    fun `either half stands alone and neither leaves a stray separator`() {
        org.junit.Assert.assertEquals("RTL Altitude", parameterSubtitle("RTL Altitude", ""))
        org.junit.Assert.assertEquals("cm", parameterSubtitle("", "cm"))
        org.junit.Assert.assertEquals("", parameterSubtitle("", ""))
    }

}
