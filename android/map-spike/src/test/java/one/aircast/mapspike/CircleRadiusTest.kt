package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class CircleRadiusTest {

    private fun circle(
        radius: Double,
        minimum: Double? = null,
        maximum: Double? = null,
        metres: Double = radius,
    ) = FenceCircle(
        index = 0,
        inclusion = true,
        centre = TrackPoint(0.0, 0.0),
        radius = radius,
        radiusMinimum = minimum,
        radiusMaximum = maximum,
        radiusMetres = metres,
    )

    @Test
    fun `a circle the fact does not bound grows and shrinks freely`() {
        assertEquals(150.0, grownRadius(circle(100.0))!!, 1e-9)
        assertEquals(100.0 / 1.5, shrunkRadius(circle(100.0))!!, 1e-9)
    }

    @Test
    fun `a step past the ceiling lands on the ceiling rather than beyond it`() {
        assertEquals(120.0, grownRadius(circle(100.0, maximum = 120.0))!!, 1e-9)
    }

    @Test
    fun `a circle already at the ceiling cannot grow, so the control goes dead`() {
        assertNull(grownRadius(circle(120.0, maximum = 120.0)))
        assertNull(grownRadius(circle(200.0, maximum = 120.0)))
    }

    @Test
    fun `a step past the floor lands on the floor`() {
        assertEquals(30.0, shrunkRadius(circle(40.0, minimum = 30.0))!!, 1e-9)
    }

    @Test
    fun `a circle already at the floor cannot shrink`() {
        assertNull(shrunkRadius(circle(30.0, minimum = 30.0)))
        assertNull(shrunkRadius(circle(10.0, minimum = 30.0)))
    }

    @Test
    fun `an unbounded circle still refuses to shrink to nothing`() {
        assertNull(shrunkRadius(circle(0.0)))
    }

    @Test
    fun `the bounds are read from the served circle`() {
        val decoded = fenceCircles(
            org.json.JSONObject(
                """{"circles":[{"index":0,"inclusion":true,"radius":100.0,
                   "centre":{"latitude":1.0,"longitude":2.0},
                   "radiusMinimum":30.0,"radiusMaximum":120.0}]}""",
            ),
        )

        assertEquals(30.0, decoded[0].radiusMinimum!!, 1e-9)
        assertEquals(120.0, decoded[0].radiusMaximum!!, 1e-9)
    }

    @Test
    fun `a circle served without bounds decodes to none rather than to zero`() {
        val decoded = fenceCircles(
            org.json.JSONObject(
                """{"circles":[{"index":0,"inclusion":true,"radius":100.0,
                   "centre":{"latitude":1.0,"longitude":2.0},
                   "radiusMinimum":null,"radiusMaximum":null}]}""",
            ),
        )

        assertNull(decoded[0].radiusMinimum)
        assertNull(decoded[0].radiusMaximum)
    }

    @Test
    fun `the bounds are in the same units as the radius, not metres`() {
        // Fact.h:45 declares `min READ cookedMin`, and FactMetaData::cookedMin runs the raw
        // bounds through _rawTranslator - the unit conversion. So the bridge's min and max are
        // in DISPLAY units, the same as radius. Measured on an imperial rig: a circle of
        // radius 326.2 ft served radiusMinimum 0.328, which is 0.1 m expressed in feet. Had it
        // been metres it would have read 0.1. Converting it again multiplied the floor by 3.28.
        val feet = circle(radius = 328.084, metres = 100.0, minimum = 30.0, maximum = 1200.0)

        assertEquals("the ceiling is 1200 ft and 328 * 1.5 fits under it", 492.126, grownRadius(feet)!!, 1e-3)
        assertEquals("the floor is 30 ft and 328 / 1.5 clears it", 218.723, shrunkRadius(feet)!!, 1e-3)

        val atCeiling = circle(radius = 1200.0, metres = 365.76, maximum = 1200.0)
        assertNull("already at the ceiling in its own units", grownRadius(atCeiling))

        val nearFloor = circle(radius = 35.0, metres = 10.67, minimum = 30.0)
        assertEquals("clamped to the floor rather than to 3.28 times it", 30.0, shrunkRadius(nearFloor)!!, 1e-3)
    }

    @Test
    fun `a metric rig cannot tell the difference, which is why this survived`() {
        val metric = circle(radius = 100.0, metres = 100.0, minimum = 30.0, maximum = 120.0)
        val imperial = circle(radius = 328.084, metres = 100.0, minimum = 98.425, maximum = 393.701)

        assertEquals(
            "where the displayed radius equals the metres, converting the bounds a second time " +
                "multiplied by 1.0 and changed nothing - so the fault only ever showed on a " +
                "profile whose display unit is not the metre",
            120.0,
            grownRadius(metric)!!,
            1e-3,
        )
        assertEquals(
            "the same fence in feet: the ceiling is 393.7 ft, not 393.7 times 3.28",
            393.701,
            grownRadius(imperial)!!,
            1e-3,
        )
    }

    @Test
    fun `a circle whose metres never resolved still bounds by its own numbers`() {
        assertEquals(
            "radiusMetres is only the map ring's geometry now; the bounds never consult it, so a " +
                "circle that never resolved metres is still clamped correctly",
            150.0,
            grownRadius(circle(radius = 100.0, metres = 0.0, maximum = 150.0))!!,
            1e-3,
        )
    }
}
