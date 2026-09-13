package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ObstacleDistanceTest {

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
