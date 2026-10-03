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
        val pattern = landingPattern(4, view(whole))!!
        assertEquals("FWLandingPatternMapVisual shows the circle only with loiter-to-altitude", 0, landingLoiterFeatures(listOf(pattern)).features()!!.size)
        assertEquals(1, landingLoiterFeatures(listOf(pattern.copy(loiterToAltitude = true))).features()!!.size)

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

    @Test
    fun `a loiter item's ring is its radius whichever way it turns`() {
        val loiter = MissionItem(index = 2, sequence = 2, latitude = 47.0, longitude = 8.0, command = "", selected = false, loiterRadius = -80.0)
        val waypoint = MissionItem(index = 1, sequence = 1, latitude = 47.1, longitude = 8.0, command = "", selected = false)
        assertEquals(listOf(TrackPoint(47.0, 8.0) to 80.0), loiterRings(emptyList(), listOf(waypoint, loiter)))
    }

    @Test
    fun `the landing area is a 15 by 100 metre box on the touchdown and the glide slope runs to the approach`() {
        val landing = TrackPoint(47.0, 8.0)
        val slope = pointAt(landing, 400.0, 90.0)
        val approach = pointAt(landing, 800.0, 90.0)
        val straight = LandingPattern(index = 3, landing = landing, slopeStart = slope, finalApproach = approach, loiterRadiusMetres = null, loiterClockwise = true)
        val area = landingArea(straight)!!
        assertEquals(4, area.size)
        assertEquals("corners are the half-diagonal from touchdown", kotlin.math.hypot(7.5, 50.0), metresBetween(landing, area[0]), 0.5)
        assertEquals("without loiter-to-altitude the slope reaches the final approach", approach, glideSlope(straight)!!.last())
        assertEquals("a radius alone is not loiter-to-altitude", approach, glideSlope(straight.copy(loiterRadiusMetres = 75.0))!!.last())
        assertEquals("with it, the slope start", slope, glideSlope(straight.copy(loiterRadiusMetres = 75.0, loiterToAltitude = true))!!.last())
        val labels = landingLabels(straight.copy(heights = GlideSlopeHeights("5 m*", "52 m*", "100.0 m"))).map { it.text }
        assertEquals("FWLandingPatternMapVisual's two names and three heights", listOf("Landing Area", "Glide Slope", "5 m*", "52 m*", "100.0 m"), labels)
        assertEquals("FWLandingPatternMapVisual draws both whether or not the item is current", 2, landingAreaFeatures(listOf(straight)).features()!!.size)
        assertEquals("only the current item's labels show", 0, landingLabelFeatures(listOf(straight), selected = 2).features()!!.size)
    }

    @Test
    fun `a structure scan names its entry and exit`() {
        val scan = MissionItem(index = 2, sequence = 2, latitude = 47.0, longitude = 8.0, command = "", selected = false, kind = KIND_STRUCTURE, exit = TrackPoint(47.1, 8.0))
        assertEquals(listOf("Entry", "Exit"), structureScanLabels(listOf(scan)).map { it.text })
        assertEquals("exitCoordinateSameAsEntry: the core sends no exit, both labels sit on the entry", listOf(scan.latitude, scan.latitude), structureScanLabels(listOf(scan.copy(exit = null))).map { it.at.latitude })
    }

    @org.junit.Test
    fun `a colliding glide slope and loiter circle turn red like their QGC visuals`() {
        val slope = LandingPattern(index = 3, landing = TrackPoint(47.0, 8.0), slopeStart = TrackPoint(47.01, 8.0), finalApproach = TrackPoint(47.02, 8.0), loiterRadiusMetres = 80.0, loiterClockwise = true, collides = true)
        val shapes = landingAreaFeatures(listOf(slope)).features()!!.associate { it.getStringProperty(LANDING_SHAPE_KIND) to it.getBooleanProperty(TERRAIN_COLLISION) }
        assertEquals("only the glide slope takes the collision colour", mapOf(LANDING_AREA_KIND to false, GLIDE_SLOPE_KIND to true), shapes)
        val loiter = MissionItem(4, 4, 47.0, 8.0, "Loiter", false, 50.0, loiterRadius = 80.0, terrainCollision = true)
        assertEquals(true, landingLoiterFeatures(emptyList(), listOf(loiter)).features()!!.single().getBooleanProperty(TERRAIN_COLLISION))
    }

    @org.junit.Test
    fun `a VTOL landing pattern draws no landing area or glide slope, as VTOLLandingPatternMapVisual`() {
        val vtol = LandingPattern(index = 3, landing = TrackPoint(47.0, 8.0), slopeStart = TrackPoint(47.01, 8.0), finalApproach = TrackPoint(47.02, 8.0), loiterRadiusMetres = 80.0, loiterClockwise = true, glideSlopeShown = false, heights = GlideSlopeHeights("1", "2", "3"))
        assertEquals(0, landingAreaFeatures(listOf(vtol)).features()!!.size)
        assertEquals(emptyList<LandingLabel>(), landingLabels(vtol))
    }
}
