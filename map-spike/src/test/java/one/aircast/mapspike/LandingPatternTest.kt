package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class LandingPatternTest {

    private fun view(body: String) = JSONObject(body)

    private val whole = """
        {"landing":{"latitude":41.70,"longitude":44.82,"altitude":0},
         "slopeStart":{"latitude":41.71,"longitude":44.83,"altitude":40},
         "finalApproach":{"latitude":41.72,"longitude":44.84,"altitude":60},
         "loiterRadiusMetres":75.0,"loiterClockwise":true}
    """

    @Test
    fun `a pattern is drawn from approach through slope start to the landing`() {
        val pattern = landingPattern(4, view(whole))!!

        assertEquals(
            listOf(pattern.finalApproach, pattern.slopeStart, pattern.landing),
            approachPath(pattern),
        )
        assertEquals(1, landingPathFeatures(listOf(pattern)).features()!!.size)
    }

    @Test
    fun `an unset corner is absent, so the pattern is never plotted at null island`() {
        val partial = landingPattern(
            4,
            view("""{"landing":null,"slopeStart":null,
                     "finalApproach":{"latitude":41.72,"longitude":44.84,"altitude":60},
                     "loiterRadiusMetres":75.0}"""),
        )!!

        assertNull(partial.landing)
        assertEquals(listOf(partial.finalApproach), approachPath(partial))
    }

    @Test
    fun `a pattern with one place draws no path, having nothing to join`() {
        val one = landingPattern(
            4,
            view("""{"finalApproach":{"latitude":41.72,"longitude":44.84,"altitude":60}}"""),
        )!!

        assertEquals(0, landingPathFeatures(listOf(one)).features()!!.size)
    }

    @Test
    fun `the loiter circle is drawn around the approach, and only when it has a radius`() {
        assertEquals(1, landingLoiterFeatures(listOf(landingPattern(4, view(whole))!!)).features()!!.size)

        val noRadius = landingPattern(
            4,
            view("""{"finalApproach":{"latitude":41.72,"longitude":44.84,"altitude":60},
                     "landing":{"latitude":41.70,"longitude":44.82,"altitude":0}}"""),
        )!!

        assertEquals(0, landingLoiterFeatures(listOf(noRadius)).features()!!.size)
    }

    @Test
    fun `an item with no pattern is not one, however the core says so`() {
        assertNull(landingPattern(4, null))
        assertNull(landingPattern(4, view("""{"kind":"null"}""")))
        assertNull(landingPattern(4, view("""{"reason":"only a fixed wing or a VTOL gets one"}""")))
        assertNull(landingPattern(4, view("""{"loiterRadiusMetres":75.0}""")))
    }
}
