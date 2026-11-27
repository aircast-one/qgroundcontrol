package one.aircast.map

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class FlyMapFitTest {
    @Test
    fun `the fly map fits whenever a new mission arrives from the vehicle, as onNewItemsFromVehicle does`() {
        val same = { it: Int -> it }
        assertTrue(missionArrived(emptyList(), listOf(1, 2), same))
        assertTrue("a replaced mission refits, as onNewItemsFromVehicle fits every download with items", missionArrived(listOf(1), listOf(1, 2), same))
        assertFalse("an unchanged mission does not refit and fight the pilot's pan", missionArrived(listOf(1, 2), listOf(1, 2), same))
        assertFalse(missionArrived(listOf(1), emptyList<Int>(), same))
    }


    @Test
    fun `the map centres once on the operator when no vehicle has a position, as FlightMap does on the first GCS fix`() {
        val here = TrackPoint(47.0, 8.0)
        assertTrue(centresOnOperator(alreadyCentred = false, operator = here, vehiclePlaced = false))
        assertFalse(centresOnOperator(alreadyCentred = true, operator = here, vehiclePlaced = false))
        assertFalse(centresOnOperator(alreadyCentred = false, operator = here, vehiclePlaced = true))
        assertFalse(centresOnOperator(alreadyCentred = false, operator = null, vehiclePlaced = false))
    }

    @Test
    fun `a map still showing the whole world opens on the home, else the launch of the flown mission, else nothing it does not know`() {
        val home = TrackPoint(41.72, 44.88)
        val launch = TrackPoint(41.7, 44.8)
        val items = listOf(MissionItem(0, 0, launch.latitude, launch.longitude, "Launch", false), MissionItem(1, 1, 41.8, 44.9, "Waypoint", false))
        assertTrue(showsWholeWorld(0.0))
        assertFalse(showsWholeWorld(15.0))
        org.junit.Assert.assertEquals(launch, launchOf(items))
        org.junit.Assert.assertEquals(home, firstKnown(listOf(home, launch)))
        org.junit.Assert.assertEquals("an unset home reads 0, 0, which is no place", launch, firstKnown(listOf(null, TrackPoint(0.0, 0.0), launch)))
        org.junit.Assert.assertNull(firstKnown(listOf(null, null)))
        org.junit.Assert.assertNull(launchOf(items.drop(1)))
    }

    @Test
    fun `the map goes to the home or launch once, only when it opened on the whole world with no remembered view and no aircraft on it`() {
        assertTrue(centresOnLaunch(alreadyCentred = false, remembered = false, zoom = 0.4, vehiclePlaced = false))
        assertFalse("a map that has not reported its zoom yet is not known to show the whole world", centresOnLaunch(alreadyCentred = false, remembered = false, zoom = null, vehiclePlaced = false))
        assertFalse("a remembered view goes first", centresOnLaunch(alreadyCentred = false, remembered = true, zoom = 0.4, vehiclePlaced = false))
        assertFalse("zooming back out to the world later does not snap to the launch again", centresOnLaunch(alreadyCentred = true, remembered = false, zoom = 0.4, vehiclePlaced = false))
        assertFalse("an aircraft with a position is followed instead", centresOnLaunch(alreadyCentred = false, remembered = false, zoom = 0.4, vehiclePlaced = true))
        assertFalse(centresOnLaunch(alreadyCentred = false, remembered = false, zoom = 12.0, vehiclePlaced = false))
    }

    @Test
    fun `other vehicles' missions come from the fly view, as FlyViewMap repeats PlanMapItems for every vehicle`() {
        val view = org.json.JSONObject("""{"items":[],"others":[{"linksStartToHome":true,"items":[]},{"items":[]}]}""")
        org.junit.Assert.assertEquals(listOf(true, false), otherMissions(view).map { it.linkStartToHome })
        org.junit.Assert.assertEquals(emptyList<OtherMission>(), otherMissions(null))
    }
}
