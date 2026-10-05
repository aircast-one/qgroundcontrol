package one.aircast.map

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ElevationCreditTest {
    @Test
    fun `the terrain profile credits its elevation provider like PlanView`() {
        assertEquals("Powered by Copernicus", elevationCredit("Copernicus"))
        assertNull(elevationCredit(""))
    }
}
