package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class TransectMarksTest {
    private fun lawnmower(transects: Int, turnaround: Boolean): List<TrackPoint> =
        (0 until transects).flatMap { row ->
            val latitude = 47.0 + row * 0.001
            val ends = listOf(TrackPoint(latitude, 8.0), TrackPoint(latitude, 8.01)).let { if (row % 2 == 0) it else it.reversed() }
            if (turnaround) listOf(ends[0], ends[0], ends[1], ends[1]) else ends
        }

    @Test
    fun `the current survey shows entry and exit arrows, two each once there are more than three transects`() {
        assertEquals(2, transectMarks(lawnmower(3, false), turnaround = false, current = true).arrows.size)
        val many = transectMarks(lawnmower(5, false), turnaround = false, current = true)
        assertEquals(4, many.arrows.size)
        assertEquals(90.0, many.arrows.first().bearing, 1.0)
        assertTrue(many.stubs.isEmpty())
    }

    @Test
    fun `another survey with turnarounds shows only its entry and exit stubs`() {
        val marks = transectMarks(lawnmower(4, true), turnaround = true, current = false)
        assertTrue(marks.arrows.isEmpty())
        assertEquals(2, marks.stubs.size)
        assertTrue(transectMarks(lawnmower(4, false), turnaround = false, current = false).stubs.isEmpty())
    }

    private fun leg(index: Int, complex: Boolean = false) = MissionItem(
        index = index, sequence = index, latitude = 47.0 + index * 0.001, longitude = 8.0, command = "", selected = false, complexPattern = complex,
    )

    @Test
    fun `mission legs carry arrows on the second leg, every sixth after, and at pattern boundaries`() {
        val plain = (0..14).map { leg(it) }
        val arrows = legArrows(plain, linkStartToHome = true)
        assertEquals("MissionController: the home-to-first leg has none, one when the count passes five, and the last leg always", 2, arrows.size)
        val withSurvey = (0..4).map { leg(it, complex = it == 3) }
        assertEquals("both legs touching a pattern get one", 2, legArrows(withSurvey, linkStartToHome = true).size)
        assertTrue("arrows sit three quarters along the leg", arrows.first().at.latitude > 47.007 && arrows.first().at.latitude < 47.008)
    }

    @Test
    fun `a waypoint's gimbal wedge points at its heading plus the gimbal yaw`() {
        val looking = leg(2).copy(heading = 90.0, gimbalYaw = 30.0)
        val bare = leg(3).copy(heading = 90.0)
        assertEquals(listOf(120.0), gimbalWedges(listOf(looking, bare)).map { it.bearing })
    }
}
