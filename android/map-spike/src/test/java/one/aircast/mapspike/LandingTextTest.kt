package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class LandingTextTest {

    private fun pattern(radius: String, clockwise: Boolean = true, index: Int = 3) = LandingPattern(
        index = index,
        landing = TrackPoint(41.0, 44.0),
        slopeStart = null,
        finalApproach = null,
        loiterRadiusMetres = 75.0,
        loiterClockwise = clockwise,
        loiterRadiusText = radius,
    )

    @Test
    fun `a drawn circle says how wide it is and which way round`() {
        assertEquals("circles 75.0 m clockwise", landingText(pattern("75.0 m")))
        assertEquals("circles 75.0 m anticlockwise", landingText(pattern("75.0 m", clockwise = false)))
    }

    @Test
    fun `a pattern the core gave no radius for says nothing rather than circling zero`() {
        assertNull(landingText(pattern("")))
        assertNull(landingText(null))
    }

    @Test
    fun `a landing picked from the list is the same one as picked by its touchdown`() {
        val landings = listOf(pattern("75.0 m", index = 3))

        assertEquals(3, selectedLanding(MapHit.Waypoint(3), landings)!!.index)
        assertEquals(
            selectedLanding(MapHit.LandingPlace(3, LANDING_PLACE_TOUCHDOWN), landings),
            selectedLanding(MapHit.Waypoint(3), landings),
        )
        assertNull(selectedLanding(MapHit.Waypoint(9), landings))
        assertNull(selectedLanding(null, landings))
    }
}
