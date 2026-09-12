package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ScanPatternsTest {

    private val served = JSONObject(
        """{"kind":"object","kinds":[
             {"id":"waypoint","simple":true,"complexName":null,"enabled":true},
             {"id":"roi","simple":true,"complexName":null,"enabled":true},
             {"id":"survey","simple":false,"complexName":"Survey","enabled":true,"disabledReason":""},
             {"id":"corridor","simple":false,"complexName":"Corridor Scan","enabled":true,"disabledReason":""},
             {"id":"structure","simple":false,"complexName":"Structure Scan","enabled":false,
              "disabledReason":"This mission starts from the ground, so a takeoff has to come first."}]}""",
    )

    @Test
    fun `only the patterns are offered, and by the core's own names`() {
        val patterns = scanPatterns(served)

        assertEquals(listOf("survey", "corridor", "structure"), patterns.map { it.id })
        assertEquals(listOf("Survey", "Corridor Scan", "Structure Scan"), patterns.map { it.label })
    }

    @Test
    fun `a pattern the plan will not take is offered disabled, with the core's reason`() {
        val structure = scanPatterns(served).single { it.id == "structure" }

        assertFalse(structure.enabled)
        assertTrue(structure.disabledReason.contains("takeoff"))
        assertTrue(scanPatterns(served).single { it.id == "survey" }.enabled)
    }

    @Test
    fun `a fourth pattern the core adds is offered without this head being changed`() {
        val later = JSONObject(
            """{"kind":"object","kinds":[
                 {"id":"spiral","simple":false,"complexName":"Spiral Scan","enabled":true}]}""",
        )

        assertEquals(listOf("Spiral Scan"), scanPatterns(later).map { it.label })
    }

    @Test
    fun `no answer offers nothing rather than a list this head remembers`() {
        assertEquals(emptyList<MissionKind>(), scanPatterns(null))
        assertEquals(emptyList<MissionKind>(), scanPatterns(JSONObject("""{"kind":"null"}""")))
    }
}
