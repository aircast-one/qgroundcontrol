package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ObstacleDistanceTest {
    private fun ring(vararg pairs: Pair<Int, Int>): List<Int> {
        val slots = MutableList(72) { 65535 }
        pairs.forEach { (index, cm) -> slots[index] = cm }
        return slots
    }

    @Test
    fun `no readings means no obstacle`() {
        assertNull(nearestObstacle(ring(), 5.0, 0.0, 20, 4000))
    }

    @Test
    fun `the nearest valid reading wins and carries its bearing`() {
        val found = nearestObstacle(ring(0 to 900, 18 to 320, 36 to 1500), 5.0, 0.0, 20, 4000)

        assertEquals(3.2, found!!.metres, 1e-9)
        assertEquals(90.0, found.bearingDeg, 1e-9)
    }

    @Test
    fun `readings outside the sensor range are not obstacles`() {
        assertNull(nearestObstacle(ring(0 to 10, 5 to 9000), 5.0, 0.0, 20, 4000))
    }

    @Test
    fun `the angle offset rotates the bearing`() {
        val found = nearestObstacle(ring(0 to 500), 5.0, 180.0, 20, 4000)

        assertEquals(180.0, found!!.bearingDeg, 1e-9)
    }

    @Test
    fun `a nonsensical increment yields nothing rather than dividing by it`() {
        assertNull(nearestObstacle(ring(0 to 500), 0.0, 0.0, 20, 4000))
    }

    @Test
    fun `each sector is named from the bearing`() {
        assertEquals("ahead", bearingSector(0.0))
        assertEquals("ahead", bearingSector(359.0))
        assertEquals("right", bearingSector(90.0))
        assertEquals("behind", bearingSector(180.0))
        assertEquals("left", bearingSector(270.0))
        assertEquals("ahead left", bearingSector(315.0))
    }

    @Test
    fun `the label reads as a distance and a direction`() {
        assertEquals("3.2 m right", obstacleLabel(Obstacle(3.2, 90.0)))
        assertNull(obstacleLabel(null))
    }

    @Test
    fun `a reading older than the stale window is not shown`() {
        assertEquals(false, obstacleIsStale(0L))
        assertEquals(false, obstacleIsStale(OBSTACLE_STALE_MS))
        assertEquals(true, obstacleIsStale(OBSTACLE_STALE_MS + 1))
    }

    @Test
    fun `never having heard from the sensor is stale, not fresh`() {
        assertEquals(true, obstacleIsStale(-1L))
    }
}
