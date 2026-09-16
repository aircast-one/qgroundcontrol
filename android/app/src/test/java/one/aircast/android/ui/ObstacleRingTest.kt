package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ObstacleRingTest {

    private fun ring(nonNull: Map<Int, Double>, size: Int = 72): String =
        (0 until size).joinToString(",") { nonNull[it]?.toString() ?: "null" }

    private fun served(
        available: Boolean = true,
        stale: Boolean = false,
        increment: String = "5.0",
        offset: String = "0.0",
        max: String = "40.0",
        body: String = ring(mapOf(18 to 3.2)),
    ) = JSONObject(
        """{"kind":"object","class":"ObstacleDistance","available":$available,"stale":$stale,
            "ringMetres":[$body],"ringIncrement":$increment,"ringOffset":$offset,
            "rangeMinMetres":0.2,"rangeMaxMetres":$max}""",
    )

    @Test
    fun `a sector's index places it, and it agrees with nearest`() {
        val read = obstacleRing(served())!!

        assertEquals(1, read.samples.size)
        assertEquals(
            "measured on the rig: sector 18 at 5 degrees a step from an offset of zero is due " +
                "right, which is exactly what nearest.bearing reported on the same read",
            90.0,
            read.samples.single().bearingDegrees,
            1e-9,
        )
        assertEquals(3.2, read.samples.single().metres, 1e-9)
    }

    @Test
    fun `an empty sector is absent, not an obstacle at zero`() {
        val read = obstacleRing(served(body = ring(mapOf(0 to 0.0, 36 to 7.5))))!!

        assertEquals(2, read.samples.size)
        assertEquals(
            "zero is a real reading - the sensor saw something at the aircraft - so it is kept, " +
                "while the seventy nulls around it are dropped",
            listOf(0.0 to 0.0, 180.0 to 7.5),
            read.samples.map { it.bearingDegrees to it.metres },
        )
    }

    @Test
    fun `a ring with no spacing cannot be placed, so it is not drawn`() {
        assertNull(
            "without an increment a sample has no angle; drawing it anywhere would invent a " +
                "direction the vehicle never reported",
            obstacleRing(served(increment = "null")),
        )
    }

    @Test
    fun `nothing is drawn when the view says there is nothing to draw`() {
        assertNull(obstacleRing(served(available = false)))
        assertNull(obstacleRing(null))
        assertNull("every sector empty is no arc", obstacleRing(served(body = ring(emptyMap()))))
    }

    @Test
    fun `a stale ring still has its samples, and says it is stale`() {
        val read = obstacleRing(served(stale = true))!!

        assertTrue(
            "measured at t+111s: the ring, nearest, sectors and available were all unchanged " +
                "67 seconds after the sensor stopped - stale is the only thing that moved, so " +
                "the drawing has to carry it or the arc claims a live obstacle",
            read.stale,
        )
        assertEquals(1, read.samples.size)
    }

    @Test
    fun `a live ring is not marked stale`() {
        assertNotNull(obstacleRing(served()))
        assertEquals(false, obstacleRing(served())!!.stale)
    }

    @Test
    fun `the ceiling comes from the vehicle, not from a guess`() {
        assertEquals(40.0, obstacleRing(served())!!.maxMetres, 1e-9)
        assertNull("no ceiling means no scale to draw against", obstacleRing(served(max = "null")))
    }

    @Test
    fun `the closest obstacle is never the least visible`() {
        val ceiling = 40.0

        assertTrue(
            "measured on the rig: 3.2 m against a 40 m ceiling is 8 percent of the radius, " +
                "which on a 48dp widget is five pixels from the centre dot - the obstacle an " +
                "operator most needs to see drawn as nothing at all",
            arcRadiusFraction(3.2, ceiling) >= NEAREST_VISIBLE_FRACTION,
        )
        assertTrue(
            "and it still grows with distance, so the picture stays honest about which is further",
            arcRadiusFraction(30.0, ceiling) > arcRadiusFraction(3.2, ceiling),
        )
        assertEquals("the ceiling is the rim", 1f, arcRadiusFraction(40.0, ceiling), 1e-6f)
        assertEquals("beyond the ceiling is still the rim", 1f, arcRadiusFraction(90.0, ceiling), 1e-6f)
        assertEquals("no ceiling, nothing to scale against", 0f, arcRadiusFraction(3.2, 0.0), 1e-6f)
    }

    @Test
    fun `the near band does not collapse, where the difference matters most`() {
        val ceiling = 40.0

        assertTrue(
            "a linear scale put 0.2 m and 3.2 m at 0.5 and 8 percent, and a floor then made " +
                "them the same mark - 0.2 m is a strike and 3.2 m is a manoeuvre, so that is " +
                "the one place the picture must not flatten",
            arcRadiusFraction(3.2, ceiling) > arcRadiusFraction(0.2, ceiling) * 2f,
        )
        assertTrue(arcRadiusFraction(1.0, ceiling) > arcRadiusFraction(0.2, ceiling))
    }

    @Test
    fun `too near is the vehicle's own judgement, not a pixel threshold`() {
        assertTrue(
            "inside twice the sensor's rated floor is the vehicle saying too near, and it " +
                "survives wherever the radius has to floor out",
            sampleIsClose(0.2, 0.2),
        )
        assertFalse(sampleIsClose(0.5, 0.2))
        assertFalse(
            "with no rated floor nothing is near, and it needs no guard of its own: the ring " +
                "drops any sample below zero, so a distance can never be under twice nothing",
            sampleIsClose(0.1, 0.0),
        )
    }

    @Test
    fun `a close reading never looks like a distant one`() {
        assertNotEquals(
            "the arc drew both in the same colour while live, because the near branch and the " +
                "default branch both resolved to the error colour - a distinction that was " +
                "only ever visible when the reading was stale, which is when it matters least",
            arcTone(near = true, stale = false),
            arcTone(near = false, stale = false),
        )
    }

    @Test
    fun `a stale reading looks like neither`() {
        assertEquals(ArcTone.Stale, arcTone(near = true, stale = true))
        assertEquals(ArcTone.Stale, arcTone(near = false, stale = true))
        assertNotEquals(ArcTone.Stale, arcTone(near = true, stale = false))
        assertNotEquals(ArcTone.Stale, arcTone(near = false, stale = false))
    }

    @Test
    fun `a sector's wedge is narrower than its spacing so neighbours stay apart`() {
        assertTrue(arcSweep(5.0) < 5.0f)
        assertTrue(arcSweep(5.0) > 0f)
    }
}