package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class MissionModelTest {
    @Test
    fun `waypoints are added in order with rising ids`() {
        val mission = MissionModel()
        val first = mission.add(41.0, 44.0)
        val second = mission.add(41.1, 44.1)

        assertNotNull(first)
        assertNotNull(second)
        assertEquals(2, mission.size)
        assertTrue(second!!.id > first!!.id)
        assertEquals(listOf(1 to first, 2 to second), mission.sequence())
    }

    @Test
    fun `an unusable position is not added`() {
        val mission = MissionModel()

        assertNull(mission.add(0.0, 0.0))
        assertNull(mission.add(Double.NaN, 44.0))
        assertNull(mission.add(91.0, 44.0))
        assertEquals(0, mission.size)
    }

    @Test
    fun `a dragged waypoint keeps its id and place in the order`() {
        val mission = MissionModel()
        val first = mission.add(41.0, 44.0)!!
        val second = mission.add(41.1, 44.1)!!

        assertTrue(mission.move(first.id, 42.0, 45.0))

        assertEquals(2, mission.size)
        assertEquals(Waypoint(first.id, 42.0, 45.0), mission.find(first.id))
        assertEquals(listOf(first.id, second.id), mission.waypoints().map { it.id })
    }

    @Test
    fun `dragging somewhere unusable is refused and changes nothing`() {
        val mission = MissionModel()
        val point = mission.add(41.0, 44.0)!!

        assertFalse(mission.move(point.id, Double.NaN, 44.0))
        assertFalse(mission.move(point.id, 0.0, 0.0))
        assertEquals(point, mission.find(point.id))
    }

    @Test
    fun `dragging an unknown waypoint is refused`() {
        val mission = MissionModel()
        mission.add(41.0, 44.0)

        assertFalse(mission.move(999, 42.0, 45.0))
        assertEquals(1, mission.size)
    }

    @Test
    fun `a removed waypoint leaves the rest renumbered`() {
        val mission = MissionModel()
        val first = mission.add(41.0, 44.0)!!
        val second = mission.add(41.1, 44.1)!!
        val third = mission.add(41.2, 44.2)!!

        assertTrue(mission.remove(second.id))

        assertEquals(2, mission.size)
        assertEquals(listOf(1 to first, 2 to third), mission.sequence())
        assertNull(mission.find(second.id))
    }

    @Test
    fun `removing an unknown waypoint reports nothing removed`() {
        val mission = MissionModel(listOf(Waypoint(1, 41.0, 44.0)))

        assertFalse(mission.remove(999))
        assertEquals(1, mission.size)
    }

    @Test
    fun `ids do not collide with a preloaded mission`() {
        val mission = MissionModel(listOf(Waypoint(7, 41.0, 44.0)))
        val added = mission.add(41.1, 44.1)!!

        assertTrue(added.id > 7)
    }
}
