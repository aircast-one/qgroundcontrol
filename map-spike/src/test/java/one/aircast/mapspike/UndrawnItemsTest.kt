package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class UndrawnItemsTest {
    private fun plan(vararg elements: String) =
        JSONObject("""{"kind":"object","elements":[${elements.joinToString(",")}]}""")

    private fun simple(name: String = "Waypoint") =
        """{"isSimpleItem":true,"commandName":"$name"}"""

    private fun complex(name: String, survey: Boolean = false) =
        """{"isSimpleItem":false,"isSurveyItem":$survey,"commandName":"$name"}"""

    private fun survey(index: Int) =
        Survey(index, listOf(TrackPoint(41.0, 44.0)), emptyList(), 0)

    @Test
    fun `a plan of simple items hides nothing`() {
        assertTrue(undrawnComplexItems(plan(simple(), simple()), emptyList(), emptyList()).isEmpty())
    }

    @Test
    fun `a survey that is drawn is not reported`() {
        val json = plan(simple(), complex("Survey", survey = true))

        assertTrue(undrawnComplexItems(json, emptyList(), listOf(survey(1))).isEmpty())
    }

    @Test
    fun `a complex item this map cannot draw is named`() {
        val json = plan(simple(), complex("Corridor Scan"), complex("Structure Scan"))

        assertEquals(
            listOf("Corridor Scan", "Structure Scan"),
            undrawnComplexItems(json, emptyList(), emptyList()),
        )
    }

    @Test
    fun `a survey the map failed to draw is reported rather than hidden`() {
        val json = plan(complex("Survey", survey = true))

        assertEquals(listOf("Survey"), undrawnComplexItems(json, emptyList(), emptyList()))
    }

    @Test
    fun `a complex item that is drawn as a marker is not reported`() {
        val json = plan(complex("Mission Start"), simple())
        val drawn = listOf(MissionItem(0, 1, 41.0, 44.0, "Mission Start", false, Double.NaN))

        assertTrue(undrawnComplexItems(json, drawn, emptyList()).isEmpty())
    }

    @Test
    fun `repeats are named once`() {
        val json = plan(complex("Corridor Scan"), complex("Corridor Scan"))

        assertEquals(listOf("Corridor Scan"), undrawnComplexItems(json, emptyList(), emptyList()))
    }

    @Test
    fun `the label is empty when everything is drawn`() {
        assertEquals("", undrawnLabel(emptyList()))
        assertEquals(" · Corridor Scan not drawn", undrawnLabel(listOf("Corridor Scan")))
    }
}
