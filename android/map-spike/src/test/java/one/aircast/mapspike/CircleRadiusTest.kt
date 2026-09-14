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
    fun `the bounds are metres and the radius is whatever the operator reads`() {
        val feet = circle(radius = 328.084, metres = 100.0, minimum = 30.0, maximum = 120.0)

        assertEquals(120.0 * 3.28084, grownRadius(feet)!!, 1e-3)
        assertEquals(328.084 / 1.5, shrunkRadius(feet)!!, 1e-3)
        assertNull(grownRadius(circle(radius = 393.7, metres = 120.0, maximum = 120.0)))
    }

    @Test
    fun `a circle whose metres never resolved is measured in its own numbers`() {
        assertEquals(1.0, shownPerMetre(circle(radius = 100.0, metres = 0.0)), 1e-9)
    }
}
