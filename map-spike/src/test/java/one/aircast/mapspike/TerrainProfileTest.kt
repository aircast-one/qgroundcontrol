package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ProfileShapeTest {
    private fun profile(vararg points: ProfilePoint) = TerrainProfile(points.toList())

    private fun at(distance: Double, planned: Double, terrain: Double? = null) =
        ProfilePoint(distance, terrain, planned)

    @Test
    fun `the distance is the last point's`() {
        assertEquals(600.0, profile(at(0.0, 50.0), at(600.0, 50.0)).distance, 1e-9)
        assertEquals(0.0, profile().distance, 1e-9)
    }

    @Test
    fun `the band covers the ground as well as the plan`() {
        val walked = profile(at(0.0, 150.0, 100.0), at(600.0, 150.0, 900.0))

        assertEquals(100.0, walked.lowest, 1e-9)
        assertEquals(900.0, walked.highest, 1e-9)
    }

    @Test
    fun `with no ground the band is the plan alone`() {
        val flown = profile(at(0.0, 40.0), at(600.0, 90.0))

        assertEquals(40.0, flown.lowest, 1e-9)
        assertEquals(90.0, flown.highest, 1e-9)
    }

    @Test
    fun `one ground reading is not a terrain line`() {
        assertFalse(profile(at(0.0, 50.0, 100.0), at(600.0, 50.0)).hasTerrain)
        assertTrue(profile(at(0.0, 50.0, 100.0), at(600.0, 50.0, 120.0)).hasTerrain)
    }

    @Test
    fun `coverage is the share of the route with ground at both ends`() {
        val half = profile(at(0.0, 50.0, 100.0), at(500.0, 50.0, 100.0), at(1000.0, 50.0))

        assertEquals(0.5, half.terrainCoverage, 1e-9)
        assertEquals(0.0, profile().terrainCoverage, 1e-9)
    }

    @Test
    fun `a plan flown at one altitude is flat but still drawable`() {
        val level = profile(at(0.0, 50.0), at(600.0, 50.0))

        assertTrue(level.flat)
        assertTrue(level.drawable)
        assertEquals(1.0, level.span, 1e-9)
    }

    @Test
    fun `a single point is not enough to draw`() {
        assertFalse(profile(at(0.0, 50.0)).drawable)
        assertFalse(profile().drawable)
    }
}

class ProfileLabelTest {
    private fun profile(
        vararg points: ProfilePoint,
        lowest: String = "40 m",
        band: String = "40 m to 90 m",
        distance: String = "0.50 km",
    ) = TerrainProfile(
        points.toList(),
        lowestText = lowest,
        bandText = band,
        distanceText = distance,
    )

    @Test
    fun `a level plan states one altitude rather than a range`() {
        assertEquals(
            "50 m AMSL \u00b7 6.43 km \u00b7 ground height unknown",
            profileLabel(
                profile(
                    ProfilePoint(0.0, null, 50.0),
                    ProfilePoint(6430.0, null, 50.0),
                    lowest = "50 m",
                    distance = "6.43 km",
                ),
            ),
        )
    }

    @Test
    fun `a climbing plan states the band the core spelled`() {
        assertEquals(
            "40 m to 90 m AMSL \u00b7 0.50 km \u00b7 ground height unknown",
            profileLabel(profile(ProfilePoint(0.0, null, 40.0), ProfilePoint(500.0, null, 90.0))),
        )
    }

    @Test
    fun `the band carries whatever unit the core spelled it in`() {
        assertEquals(
            "131 ft to 295 ft AMSL \u00b7 1640 ft \u00b7 ground height unknown",
            profileLabel(
                profile(
                    ProfilePoint(0.0, null, 40.0),
                    ProfilePoint(500.0, null, 90.0),
                    band = "131 ft to 295 ft",
                    distance = "1640 ft",
                ),
            ),
        )
    }

    @Test
    fun `terrain at points that do not join up is not one per cent of the route`() {
        val measured = profile(
            ProfilePoint(0.0, 440.0, 440.0),
            ProfilePoint(0.0, null, 490.0),
            ProfilePoint(36.0, 444.0, 490.0),
            band = "430 m to 500 m",
            distance = "36 m",
        )
        assertEquals(0.0, measured.terrainCoverage, 0.0)
        assertEquals(
            "430 m to 500 m AMSL \u00b7 36 m \u00b7 ground height at points, none along the route",
            profileLabel(measured),
        )
    }

    @Test
    fun `ground under part of the route says how much`() {
        assertEquals(
            "40 m to 90 m AMSL \u00b7 1.00 km \u00b7 ground height for 50% of the route",
            profileLabel(
                profile(
                    ProfilePoint(0.0, 40.0, 40.0),
                    ProfilePoint(500.0, 45.0, 65.0),
                    ProfilePoint(1000.0, null, 90.0),
                    distance = "1.00 km",
                ),
            ),
        )
    }

    @Test
    fun `a sliver of ground is never reported as none of the route`() {
        assertEquals(
            "40 m to 90 m AMSL \u00b7 1.00 km \u00b7 ground height for 1% of the route",
            profileLabel(
                profile(
                    ProfilePoint(0.0, 40.0, 40.0),
                    ProfilePoint(1.0, 41.0, 41.0),
                    ProfilePoint(1000.0, null, 90.0),
                    distance = "1.00 km",
                ),
            ),
        )
    }

    @Test
    fun `known ground height drops the caveat`() {
        assertEquals(
            "40 m to 90 m AMSL \u00b7 0.50 km",
            profileLabel(profile(ProfilePoint(0.0, 40.0, 40.0), ProfilePoint(500.0, 45.0, 90.0))),
        )
    }
}

class TerrainViewTest {
    private fun view(vararg points: String) =
        JSONObject("""{"kind":"object","class":"TerrainProfile","points":[${points.joinToString(",")}]}""")

    private fun at(distance: Double, planned: Double, terrain: String) =
        """{"distance":$distance,"missionAltitude":$planned,"terrainAltitude":$terrain}"""

    @Test
    fun `the profile is read straight from the core's points`() {
        val profile = terrainProfile(view(at(0.0, 50.0, "948.0"), at(200.0, 100.0, "1000.0")))

        assertEquals(listOf(0.0, 200.0), profile.points.map { it.distance })
        assertEquals(listOf(50.0, 100.0), profile.points.map { it.planned })
        assertEquals(listOf(948.0, 1000.0), profile.points.map { it.terrain })
        assertEquals(200.0, profile.distance, 1e-9)
    }

    @Test
    fun `ground the core could not resolve stays unknown rather than becoming zero`() {
        val profile = terrainProfile(view(at(0.0, 50.0, "null"), at(200.0, 100.0, "1000.0")))

        assertEquals(listOf(null, 1000.0), profile.points.map { it.terrain })
        assertEquals(50.0, profile.lowest, 1e-9)
    }

    @Test
    fun `a point with no planned altitude is not a point on the profile`() {
        val profile = terrainProfile(view("""{"distance":10.0,"terrainAltitude":900.0}"""))

        assertEquals(0, profile.points.size)
    }

    @Test
    fun `an absent or empty view draws nothing`() {
        assertEquals(0, terrainProfile(null).points.size)
        assertFalse(terrainProfile(view()).drawable)
    }
}

class HeightRangeTest {
    private fun profile(
        low: Double,
        high: Double,
        lowText: String = "",
        highText: String = "",
        bandText: String = "",
    ) = TerrainProfile(
        points = listOf(
            ProfilePoint(0.0, null, low),
            ProfilePoint(100.0, null, high),
        ),
        lowestText = lowText,
        highestText = highText,
        bandText = bandText,
    )

    @Test
    fun `the band is spelled by the core, which gives both ends one precision`() {
        assertEquals(
            "-33 ft to 197 ft AMSL",
            heightRange(profile(-10.0, 60.0, "-32.8 ft", "197 ft", "-33 ft to 197 ft")),
        )
    }

    @Test
    fun `a flat route names one height, not a range of one`() {
        assertEquals("50 m AMSL", heightRange(profile(50.0, 50.0, "50 m", "50 m", "50 m to 50 m")))
    }

}
