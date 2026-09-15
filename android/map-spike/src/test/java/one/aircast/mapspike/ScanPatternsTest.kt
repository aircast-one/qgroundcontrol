package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ScanPatternsTest {
    @Test
    fun `a long press is refused where every button for the same kind is`() {
        val kinds = missionKinds(
            JSONObject(
                """{"kinds":[
                  {"id":"takeoff","simple":true,"enabled":true},
                  {"id":"waypoint","simple":true,"enabled":false,
                   "disabledReason":"This mission starts from the ground, so a takeoff has to come before anything else."}]}""",
            ),
        )

        assertFalse(
            "missionkinds.rs refuses every kind but takeoff on an empty ground-start mission, and " +
                "the Survey, ROI and Land buttons honour that. The map's long press added a " +
                "waypoint regardless - and the summary line telling the operator to long press is " +
                "the most prominent instruction on the screen",
            kindAllows(kinds, KIND_WAYPOINT),
        )
        assertTrue(kindAllows(kinds, KIND_TAKEOFF))
    }



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
        val patterns = scanPatterns(missionKinds(served))

        assertEquals(listOf("survey", "corridor", "structure"), patterns.map { it.id })
        assertEquals(listOf("Survey", "Corridor Scan", "Structure Scan"), patterns.map { it.label })
    }

    @Test
    fun `a pattern the plan will not take is offered disabled, with the core's reason`() {
        val structure = scanPatterns(missionKinds(served)).single { it.id == "structure" }

        assertFalse(structure.enabled)
        assertTrue(structure.disabledReason.contains("takeoff"))
        assertTrue(scanPatterns(missionKinds(served)).single { it.id == "survey" }.enabled)
    }

    @Test
    fun `a fourth pattern the core adds is offered without this head being changed`() {
        val later = JSONObject(
            """{"kind":"object","kinds":[
                 {"id":"spiral","simple":false,"complexName":"Spiral Scan","enabled":true}]}""",
        )

        assertEquals(listOf("Spiral Scan"), scanPatterns(missionKinds(later)).map { it.label })
    }

    @Test
    fun `no answer offers nothing rather than a list this head remembers`() {
        assertEquals(emptyList<MissionKind>(), scanPatterns(missionKinds(null)))
        assertEquals(emptyList<MissionKind>(), scanPatterns(missionKinds(JSONObject("""{"kind":"null"}"""))))
    }
}

class MissionKindGateTest {

    private val empty = JSONObject(
        """{"kind":"object","kinds":[
             {"id":"takeoff","simple":true,"enabled":true,"disabledReason":""},
             {"id":"land","simple":true,"enabled":false,
              "disabledReason":"This mission starts from the ground, so a takeoff has to come first."},
             {"id":"roi","simple":true,"enabled":false,
              "disabledReason":"This mission starts from the ground, so a takeoff has to come first."}]}""",
    )

    @Test
    fun `only the kind the plan will take is offered`() {
        val kinds = missionKinds(empty)

        assertTrue(kindAllows(kinds, "takeoff"))
        assertFalse(kindAllows(kinds, "land"))
        assertFalse(kindAllows(kinds, "roi"))
    }

    @Test
    fun `a kind the core never mentioned stays offered rather than disappearing on silence`() {
        assertTrue(kindAllows(missionKinds(empty), "spiral"))
        assertTrue(kindAllows(emptyList(), "land"))
    }

    @Test
    fun `the reason is shown once, from the core, rather than beside every dead control`() {
        assertTrue(blockedReason(missionKinds(empty))!!.contains("takeoff has to come"))
        assertNull(blockedReason(emptyList()))
    }
}
