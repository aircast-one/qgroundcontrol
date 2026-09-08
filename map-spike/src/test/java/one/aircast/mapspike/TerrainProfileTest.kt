package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class TerrainProfileTest {
    private fun item(
        latitude: Double,
        longitude: Double,
        terrain: Double? = 100.0,
        planned: Double? = 150.0,
        specifies: Boolean = true,
    ): String {
        val terrainJson = terrain?.let { ""","terrainAltitude":$it""" } ?: ""
        val plannedJson = planned?.let { ""","amslEntryAlt":$it""" } ?: ""
        return """{"specifiesCoordinate":$specifies,""" +
            """"coordinate":{"latitude":$latitude,"longitude":$longitude}$terrainJson$plannedJson}"""
    }

    private val settingsItem =
        """{"specifiesCoordinate":true,"coordinate":{"latitude":41.0,"longitude":44.0},""" +
            """"amslEntryAlt":0.0}"""

    private fun model(vararg items: String) =
        JSONObject(
            """{"kind":"object","elements":[${(listOf(settingsItem) + items).joinToString(",")}]}""",
        )

    @Test
    fun `distance accumulates along the route`() {
        val profile = terrainProfile(
            model(
                item(41.0, 44.0),
                item(41.0, 44.01),
                item(41.0, 44.02),
            ),
        )

        assertEquals(3, profile.points.size)
        assertEquals(0.0, profile.points[0].distance, 0.001)
        assertTrue(profile.points[1].distance > 800.0)
        assertEquals(profile.points[1].distance * 2, profile.points[2].distance, 1.0)
    }

    @Test
    fun `a metre distance is roughly right`() {
        val oneDegreeOfLatitude = metresBetween(TrackPoint(0.0, 0.0), TrackPoint(1.0, 0.0))

        assertEquals(111_195.0, oneDegreeOfLatitude, 500.0)
        assertEquals(0.0, metresBetween(TrackPoint(41.0, 44.0), TrackPoint(41.0, 44.0)), 0.001)
    }

    @Test
    fun `an item with unknown ground height keeps its planned altitude`() {
        val profile = terrainProfile(
            model(
                item(41.0, 44.0),
                item(41.0, 44.01, terrain = null),
                item(41.0, 44.02),
            ),
        )

        assertEquals(3, profile.points.size)
        assertEquals(null, profile.points[1].terrain)
        assertEquals(150.0, profile.points[1].planned, 0.001)
    }

    @Test
    fun `a route with no ground height at all still draws its planned line`() {
        val profile = terrainProfile(
            model(
                item(41.0, 44.0, terrain = null, planned = 100.0),
                item(41.0, 44.01, terrain = null, planned = 200.0),
            ),
        )

        assertTrue(profile.drawable)
        assertFalse(profile.hasTerrain)
        assertEquals(100.0, profile.lowest, 0.001)
        assertEquals(200.0, profile.highest, 0.001)
        assertEquals(0, profileOffsets(profile, 100f, 50f) { it.terrain }.size)
        assertEquals(2, profileOffsets(profile, 100f, 50f) { it.planned }.size)
    }

    @Test
    fun `an item with no planned altitude is dropped`() {
        val profile = terrainProfile(
            model(item(41.0, 44.0), item(41.0, 44.01, planned = null), item(41.0, 44.02)),
        )

        assertEquals(2, profile.points.size)
        assertTrue(profile.points.last().distance > 1600.0)
    }

    @Test
    fun `items that carry no coordinate are skipped`() {
        val profile = terrainProfile(
            model(item(41.0, 44.0, specifies = false), item(41.0, 44.01), item(41.0, 44.02)),
        )

        assertEquals(2, profile.points.size)
    }

    @Test
    fun `range spans both the ground and the planned line`() {
        val profile = terrainProfile(
            model(
                item(41.0, 44.0, terrain = 50.0, planned = 200.0),
                item(41.0, 44.01, terrain = 300.0, planned = 120.0),
            ),
        )

        assertEquals(50.0, profile.lowest, 0.001)
        assertEquals(300.0, profile.highest, 0.001)
        assertTrue(profile.drawable)
    }

    @Test
    fun `a single point is not drawable`() {
        assertFalse(terrainProfile(model(item(41.0, 44.0))).drawable)
    }

    @Test
    fun `a flat range draws as a flat profile rather than refusing`() {
        val profile = terrainProfile(
            model(
                item(41.0, 44.0, terrain = 100.0, planned = 100.0),
                item(41.0, 44.01, terrain = 100.0, planned = 100.0),
            ),
        )

        assertTrue(profile.drawable)
        assertTrue(profile.flat)
    }

    @Test
    fun `points scale into the drawing area`() {
        val profile = terrainProfile(
            model(
                item(41.0, 44.0, terrain = 0.0, planned = 100.0),
                item(41.0, 44.01, terrain = 100.0, planned = 100.0),
            ),
        )
        val offsets = profileOffsets(profile, 200f, 100f) { it.terrain }

        assertEquals(2, offsets.size)
        assertEquals(0f, offsets.first().x, 0.001f)
        assertEquals(100f, offsets.first().y, 0.001f)
        assertEquals(200f, offsets.last().x, 0.001f)
        assertEquals(0f, offsets.last().y, 0.001f)
    }

    private fun survey(
        latitude: Double,
        longitude: Double,
        exitLatitude: Double,
        exitLongitude: Double,
        distance: Double,
        entryAlt: Double = 50.0,
        exitAlt: Double = 50.0,
    ) = """{"specifiesCoordinate":true,""" +
        """"coordinate":{"latitude":$latitude,"longitude":$longitude},""" +
        """"exitCoordinate":{"latitude":$exitLatitude,"longitude":$exitLongitude},""" +
        """"complexDistance":$distance,"amslEntryAlt":$entryAlt,"amslExitAlt":$exitAlt}"""

    @Test
    fun `a survey contributes the distance it flies rather than a single point`() {
        val profile = terrainProfile(
            model(survey(41.0, 44.0, 41.0, 44.01, distance = 6000.0)),
        )

        assertEquals(2, profile.points.size)
        assertEquals(0.0, profile.points[0].distance, 1e-6)
        assertEquals(6000.0, profile.points[1].distance, 1e-6)
        assertEquals(6000.0, profile.distance, 1e-6)
    }

    @Test
    fun `the item after a survey is measured from where the survey exits`() {
        val exitToNext = metresBetween(TrackPoint(41.0, 44.01), TrackPoint(41.0, 44.02))
        val profile = terrainProfile(
            model(
                survey(41.0, 44.0, 41.0, 44.01, distance = 6000.0),
                item(41.0, 44.02),
            ),
        )

        assertEquals(3, profile.points.size)
        assertEquals(6000.0 + exitToNext, profile.distance, 1e-6)
    }

    @Test
    fun `a survey climbs from its entry altitude to its exit altitude`() {
        val profile = terrainProfile(
            model(survey(41.0, 44.0, 41.0, 44.01, distance = 500.0, entryAlt = 40.0, exitAlt = 90.0)),
        )

        assertEquals(40.0, profile.points[0].planned, 1e-9)
        assertEquals(90.0, profile.points[1].planned, 1e-9)
    }

    @Test
    fun `a plain waypoint still contributes one point and no extra distance`() {
        val profile = terrainProfile(model(item(41.0, 44.0), item(41.0, 44.01)))

        assertEquals(2, profile.points.size)
        assertEquals(
            metresBetween(TrackPoint(41.0, 44.0), TrackPoint(41.0, 44.01)),
            profile.distance,
            1e-6,
        )
    }

}

class FlatProfileTest {
    private fun plan(vararg altitudes: Double): TerrainProfile {
        val elements = listOf("""{"specifiesCoordinate":true,"amslEntryAlt":0.0}""") +
            altitudes.mapIndexed { index, alt ->
                """{"specifiesCoordinate":true,"coordinate":{"latitude":${41.0 + index * 0.01},""" +
                    """"longitude":44.0},"amslEntryAlt":$alt}"""
            }
        return terrainProfile(
            org.json.JSONObject("""{"kind":"object","elements":[${elements.joinToString(",")}]}"""),
        )
    }

    @Test
    fun `a mission flown at one altitude is a flat profile not an absent one`() {
        val profile = plan(50.0, 50.0, 50.0)

        assertTrue(profile.drawable)
        assertTrue(profile.flat)
    }

    @Test
    fun `a flat profile still has a span so it can be plotted`() {
        assertTrue(plan(50.0, 50.0).span > 0.0)
    }

    @Test
    fun `a varying profile is not flat and keeps its real span`() {
        val profile = plan(50.0, 150.0)

        assertTrue(!profile.flat)
        assertEquals(100.0, profile.span, 1e-9)
    }

    @Test
    fun `a single point is still not enough to draw`() {
        assertTrue(!plan(50.0).drawable)
    }
}

class ProfileLabelTest {
    private fun profile(vararg points: ProfilePoint) = TerrainProfile(points.toList())

    @Test
    fun `a level plan states one altitude rather than a range`() {
        assertEquals(
            "50 m AMSL \u00b7 6.43 km \u00b7 ground height unknown",
            profileLabel(profile(ProfilePoint(0.0, null, 50.0), ProfilePoint(6430.0, null, 50.0))),
        )
    }

    @Test
    fun `a climbing plan states the range it covers`() {
        assertEquals(
            "40\u201390 m AMSL \u00b7 0.50 km \u00b7 ground height unknown",
            profileLabel(profile(ProfilePoint(0.0, null, 40.0), ProfilePoint(500.0, null, 90.0))),
        )
    }

    @Test
    fun `known ground height drops the caveat`() {
        assertEquals(
            "40\u201390 m AMSL \u00b7 0.50 km",
            profileLabel(profile(ProfilePoint(0.0, 40.0, 40.0), ProfilePoint(500.0, 45.0, 90.0))),
        )
    }
}
