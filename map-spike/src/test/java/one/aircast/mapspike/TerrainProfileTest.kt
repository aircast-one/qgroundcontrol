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

    private fun model(vararg items: String) =
        JSONObject("""{"kind":"object","elements":[${items.joinToString(",")}]}""")

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
    fun `a single point or a flat range is not drawable`() {
        assertFalse(terrainProfile(model(item(41.0, 44.0))).drawable)
        assertFalse(
            terrainProfile(
                model(
                    item(41.0, 44.0, terrain = 100.0, planned = 100.0),
                    item(41.0, 44.01, terrain = 100.0, planned = 100.0),
                ),
            ).drawable,
        )
        assertFalse(terrainProfile(null).drawable)
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
}
