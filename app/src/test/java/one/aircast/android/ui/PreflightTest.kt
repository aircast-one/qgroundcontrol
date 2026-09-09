package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class PreflightTest {

    private val served = """
        {"airframe":"Quadrotor · ArduPilot","total":4,"blocked":["Props on"],"groups":[
          {"name":"Before you fly","checks":[
            {"name":"Props on","prompt":"Propellers fitted and tight","verdict":"failing",
             "reason":"Vehicle reports it is not ready to arm.","blocked":true},
            {"name":"Area clear","prompt":"Nobody within the rotor arc","verdict":"manual",
             "reason":"","blocked":false}]},
          {"name":"Vehicle","checks":[
            {"name":"Battery","prompt":"Pack charged and secured","verdict":"passing",
             "reason":"","blocked":false},
            {"name":"Compass","prompt":"Compass calibrated","verdict":"overridable",
             "reason":"Interference detected.","blocked":false}]}]}
    """

    @Test
    fun `the groups and their checks come through with verdicts`() {
        val checks = preflight(JSONObject(served))!!

        assertEquals(listOf("Before you fly", "Vehicle"), checks.groups.map { it.name })
        assertEquals(listOf("failing", "manual"), checks.groups[0].checks.map { it.verdict })
        assertEquals(listOf("Props on"), checks.blocked)
        assertEquals(4, checks.total)
    }

    @Test
    fun `only a manual check is the operator's to tick`() {
        val checks = preflight(JSONObject(served))!!
        val manual = checks.groups[0].checks.first { it.name == "Area clear" }
        val passing = checks.groups[1].checks.first { it.name == "Battery" }

        assertTrue(checkNeedsTicking(manual))
        assertFalse(checkNeedsTicking(passing))
    }

    @Test
    fun `a failing check shows the vehicle's reason rather than a generic word`() {
        val checks = preflight(JSONObject(served))!!
        val failing = checks.groups[0].checks.first { it.name == "Props on" }
        val overridable = checks.groups[1].checks.first { it.name == "Compass" }

        assertEquals("Vehicle reports it is not ready to arm.", checkStatusText(failing, false))
        assertEquals("Interference detected.", checkStatusText(overridable, false))
        assertEquals("Passing", checkStatusText(checks.groups[1].checks[0], false))
    }

    @Test
    fun `the summary leads with what would stop the flight`() {
        val checks = preflight(JSONObject(served))!!

        assertEquals("1 of 4 will stop the flight.", preflightSummary(checks, emptySet()))
        assertEquals(
            "2 of 4 done.",
            preflightSummary(checks.copy(blocked = emptyList()), setOf("Area clear", "Props on")),
        )
        assertEquals(
            "All 4 checks done.",
            preflightSummary(checks.copy(blocked = emptyList()), setOf("a", "b", "c", "d")),
        )
    }

    @Test
    fun `no vehicle is a sentence, not an empty list`() {
        assertNull(preflight(null))
        assertEquals(
            "Connect a vehicle to run its preflight checks.",
            preflightSummary(null, emptySet()),
        )
    }
}
