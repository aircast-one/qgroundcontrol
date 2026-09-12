package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
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

class LandingHandleTest {

    private val pattern = LandingPattern(
        index = 4,
        landing = TrackPoint(41.70, 44.82),
        slopeStart = TrackPoint(41.71, 44.83),
        finalApproach = TrackPoint(41.72, 44.84),
        loiterRadiusMetres = 75.0,
        loiterClockwise = true,
    )

    @Test
    fun `only the two places QGC lets you set get a handle`() {
        val handles = vertexHandleFeatures(emptyList(), emptyList(), emptyList(), listOf(pattern))
            .features()!!

        assertEquals(2, handles.size)
        assertEquals(
            listOf(LANDING_PLACE_APPROACH, LANDING_PLACE_TOUCHDOWN),
            handles.map { it.getNumberProperty(VERTEX_INDEX_PROPERTY).toInt() },
        )
        assertEquals(
            listOf(HANDLE_KIND_LANDING, HANDLE_KIND_LANDING),
            handles.map { it.getStringProperty(HANDLE_KIND_PROPERTY) },
        )
    }

    @Test
    fun `an unplaced pattern offers nothing to drag`() {
        val empty = pattern.copy(landing = null, slopeStart = null, finalApproach = null)

        assertEquals(
            0,
            vertexHandleFeatures(emptyList(), emptyList(), emptyList(), listOf(empty)).features()!!.size,
        )
    }

    @Test
    fun `a selection on a landing place survives while the pattern does`() {
        assertTrue(
            selectionSurvives(
                MapHit.LandingPlace(4, LANDING_PLACE_APPROACH),
                emptyList(), emptyList(), emptyList(), emptyList(), emptyList(), listOf(pattern),
            ),
        )
        assertTrue(
            !selectionSurvives(
                MapHit.LandingPlace(9, LANDING_PLACE_APPROACH),
                emptyList(), emptyList(), emptyList(), emptyList(), emptyList(), listOf(pattern),
            ),
        )
    }

    @Test
    fun `each place says which one moved`() {
        assertEquals(
            "Moved the final approach",
            movedText(MapHit.LandingPlace(4, LANDING_PLACE_APPROACH), emptyList()),
        )
        assertEquals(
            "Moved the touchdown",
            movedText(MapHit.LandingPlace(4, LANDING_PLACE_TOUCHDOWN), emptyList()),
        )
    }
}

class IsLandingPatternTest {

    @Test
    fun `a refusal is not a pattern, and neither is an absent view`() {
        assertTrue(!isLandingPattern(null))
        assertTrue(!isLandingPattern(JSONObject("""{"kind":"null"}""")))
        assertTrue(
            !isLandingPattern(JSONObject("""{"reason":"only a fixed wing or a VTOL gets one"}""")),
        )
    }

    @Test
    fun `a pattern with no places is still a pattern, which is the whole distinction`() {
        val unplaced = JSONObject("""{"landing":null,"slopeStart":null,"finalApproach":null}""")

        assertTrue(isLandingPattern(unplaced))
        assertNull(landingPattern(4, unplaced))
    }
}
