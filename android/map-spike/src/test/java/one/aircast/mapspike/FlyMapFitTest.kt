package one.aircast.mapspike

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class FlyMapFitTest {
    @Test
    fun `the fly map fits once a mission arrives from the vehicle, as onNewItemsFromVehicle does`() {
        assertTrue(missionArrived(emptyList(), listOf(1, 2)))
        assertFalse("an unchanged mission does not refit and fight the pilot's pan", missionArrived(listOf(1), listOf(1, 2)))
        assertFalse(missionArrived(listOf(1), emptyList<Int>()))
    }
}
