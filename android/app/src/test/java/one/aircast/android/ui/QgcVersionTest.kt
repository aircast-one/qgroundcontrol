package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class QgcVersionTest {
    @Test
    fun versionCarriesTheBuildAbiBitnessLikeQgcVersion() {
        assertEquals("5.0.1 64 bit", qgcVersion("5.0.1", is64Bit = true))
        assertEquals("5.0.1 32 bit", qgcVersion("5.0.1", is64Bit = false))
    }
}
