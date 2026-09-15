package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ObstacleDistanceTest {
    @Test
    fun `a reading whose age the sensor never reported is still drawn, deliberately`() {
        val ageless = JSONObject(
            """{"available":true,"stale":null,"nearest":{"distanceText":"4 m","sectorText":"ahead","close":true}}""",
        )
        assertEquals(
            "obstacle.rs stale is `since.map(..)`, so null means the sensor never said how old the " +
                "reading is. Drawing it anyway is the codebase's own rule - ObstacleModel.swift:66 " +
                "says a reading must never be suppressed, because with avoidance off the operator " +
                "needs the distance MORE. This pins the choice so it is a decision and not an " +
                "accident of optBoolean flattening null to false",
            "4 m ahead",
            obstacleWarning(ageless)?.label,
        )
    }

    @Test
    fun `a reading the sensor called stale is withheld`() {
        val old = JSONObject(
            """{"available":true,"stale":true,"nearest":{"distanceText":"4 m","sectorText":"ahead"}}""",
        )
        assertNull(obstacleWarning(old))
    }


    private fun view(
        available: Boolean = true,
        stale: Boolean = false,
        nearest: String? = """{"distanceText":"3.2 m","sectorText":"right","close":false}""",
    ) = JSONObject(
        """{"available":$available,"stale":$stale,"nearest":${nearest ?: "null"}}""",
    )

    @Test
    fun `a reading is the core's distance and the core's sector`() {
        val warning = obstacleWarning(view())!!

        assertEquals("3.2 m right", warning.label)
        assertTrue(!warning.close)
    }

    @Test
    fun `the distance is whatever the core spelled, so it follows the operator's units`() {
        val feet = view(nearest = """{"distanceText":"10.5 ft","sectorText":"ahead","close":true}""")

        assertEquals("10.5 ft ahead", obstacleWarning(feet)!!.label)
    }

    @Test
    fun `a close reading says so, for the head to colour`() {
        val near = view(nearest = """{"distanceText":"0.2 m","sectorText":"ahead","close":true}""")

        assertTrue(obstacleWarning(near)!!.close)
    }

    @Test
    fun `nothing is shown when the sensor is absent, stale, or reports no obstacle`() {
        assertNull(obstacleWarning(null))
        assertNull(obstacleWarning(view(available = false)))
        assertNull(obstacleWarning(view(stale = true)))
        assertNull(obstacleWarning(view(nearest = null)))
    }

    @Test
    fun `a reading with no distance text is not a warning`() {
        assertNull(obstacleWarning(view(nearest = """{"sectorText":"right"}""")))
    }
}
