package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Test

class LongitudeArcTest {

    @Test
    fun `a plan either side of the antimeridian keeps the short arc across it`() {
        assertEquals(
            "sorting puts -170 first, so the naive west-to-east span is the 340 degrees the plan " +
                "does NOT occupy - the whole point of picking the widest gap is to return the 20 " +
                "that it does",
            170.0 to -170.0,
            longitudeArc(listOf(170.0, -170.0)),
        )
    }

    @Test
    fun `an ordinary plan spans west to east the obvious way`() {
        assertEquals(10.0 to 20.0, longitudeArc(listOf(20.0, 10.0)))
        assertEquals(-20.0 to -10.0, longitudeArc(listOf(-10.0, -20.0)))
    }

    @Test
    fun `one point and no points are both answerable`() {
        assertEquals(42.0 to 42.0, longitudeArc(listOf(42.0)))
        assertEquals(42.0 to 42.0, longitudeArc(listOf(42.0, 42.0)))
        assertEquals(0.0 to 0.0, longitudeArc(emptyList()))
    }

    @Test
    fun `a longitude past the wrap is brought back before anything is compared`() {
        assertEquals(180.0, normaliseLongitude(-180.0), 0.0)
        assertEquals(-179.0, normaliseLongitude(181.0), 0.0)
        assertEquals(179.0, normaliseLongitude(-181.0), 0.0)
        assertEquals(0.0, normaliseLongitude(360.0), 0.0)
    }

    @Test
    fun `three points straddling the wrap still take the arc containing them`() {
        assertEquals(
            "going east the gaps are -175 to 10 (185 degrees), 10 to 175 (165) and 175 to -175 " +
                "(10). The arc must exclude the widest, so it runs from 10 east across the " +
                "antimeridian to -175 and is 175 degrees wide - starting at 175 instead would be " +
                "195 and would not be the containing arc",
            10.0 to -175.0,
            longitudeArc(listOf(175.0, -175.0, 10.0)),
        )
    }
}
