package one.aircast.android.ui

import org.junit.Assert.assertEquals
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
}
