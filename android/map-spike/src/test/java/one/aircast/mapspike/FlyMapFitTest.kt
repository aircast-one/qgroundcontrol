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

    @Test
    fun `the item the vehicle is flying to is the one drawn current, as MissionController marks isCurrentItem`() {
        val items = listOf(MissionItem(0, 0, 47.0, 8.0, "Home", false), MissionItem(1, 1, 47.1, 8.0, "Waypoint", false), MissionItem(2, 3, 47.2, 8.0, "Waypoint", false))
        org.junit.Assert.assertEquals(2, currentItemIndex(items, 3))
        org.junit.Assert.assertEquals(null, currentItemIndex(items, -1))
        org.junit.Assert.assertEquals(null, currentItemIndex(items, 0))
    }
}
