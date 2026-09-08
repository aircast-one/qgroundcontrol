package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class OverSpanTest {
    private fun at(distance: Double) = ProfilePoint(distance, 400.0, 100.0)

    @Test
    fun `samples that fall short of the span are stretched to fill it`() {
        assertEquals(
            listOf(1000.0, 1500.0, 2000.0),
            overSpan(listOf(at(1000.0), at(1250.0), at(1500.0)), 1000.0, 1000.0)
                .map { it.distance },
        )
    }

    @Test
    fun `samples that already fill the span are left alone`() {
        assertEquals(
            listOf(1000.0, 1250.0, 1500.0),
            overSpan(listOf(at(1000.0), at(1250.0), at(1500.0)), 1000.0, 500.0)
                .map { it.distance },
        )
    }

    @Test
    fun `samples that overshoot the span are pulled back into it`() {
        assertEquals(
            listOf(0.0, 100.0, 200.0),
            overSpan(listOf(at(0.0), at(500.0), at(1000.0)), 0.0, 200.0).map { it.distance },
        )
    }

    @Test
    fun `samples with no extent are left as they are rather than divided by zero`() {
        assertEquals(
            listOf(500.0, 500.0),
            overSpan(listOf(at(500.0), at(500.0)), 500.0, 300.0).map { it.distance },
        )
    }

    @Test
    fun `the profile spans the distance the plan claims, not the sum of its segments`() {
        val plan = JSONObject(
            """
            {"kind":"object","elements":[
              {"coordinate":{"latitude":47.0,"longitude":8.0}},
              {"specifiesCoordinate":true,"isTakeoffItem":true,
               "coordinate":{"latitude":47.0,"longitude":8.0},
               "exitCoordinate":{"latitude":47.0,"longitude":8.0},
               "complexDistance":2000.0,"amslEntryAlt":600.0,"amslExitAlt":600.0}
            ]}
            """,
        )
        val short = JSONObject(
            """
            {"kind":"object","elements":[
              {"totalDistance":500.0,"amslTerrainHeights":[400.0,410.0,420.0]}
            ]}
            """,
        )

        assertEquals(2000.0, terrainProfile(plan) { short }.distance, 1e-6)
    }
}
