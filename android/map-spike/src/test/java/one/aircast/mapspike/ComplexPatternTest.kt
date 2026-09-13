package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ComplexPatternTest {

    private fun items(json: String) = allMissionItems(JSONObject(json))

    @Test
    fun `a corridor scan is a pattern even though it is not a survey`() {
        val plan = items(
            """{"kind":"object","items":[
                 {"index":0,"sequence":0,"name":"Waypoint","kind":"waypoint","simple":true},
                 {"index":1,"sequence":1,"name":"Survey","kind":"survey","simple":false},
                 {"index":2,"sequence":2,"name":"Corridor Scan","kind":"corridor","simple":false},
                 {"index":3,"sequence":3,"name":"Structure Scan","kind":"structure","simple":false}]}""",
        )

        assertFalse(plan[0].complexPattern)
        assertEquals(
            listOf(1, 2, 3),
            plan.filter { it.complexPattern }.map { it.index },
        )
    }

    @Test
    fun `an item the core said nothing about is treated as simple rather than asked about`() {
        val plan = items("""{"kind":"object","items":[{"index":0,"sequence":0,"name":"Waypoint","kind":"waypoint"}]}""")

        assertFalse(plan.single().complexPattern)
    }

    @Test
    fun `the camera question is asked of every pattern, not only the one named survey`() {
        val plan = items(
            """{"kind":"object","items":[
                 {"index":2,"sequence":2,"name":"Corridor Scan","kind":"corridor","simple":false,"cameraShots":27}]}""",
        )

        assertTrue(
            "a corridor inherits cameraCalc from TransectStyleComplexItem, so gating on the survey type hides camera work it really did",
            plan.single().complexPattern,
        )
    }
}
