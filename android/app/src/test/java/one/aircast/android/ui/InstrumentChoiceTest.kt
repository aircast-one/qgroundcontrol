package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class InstrumentChoiceTest {

    private val served = JSONObject(
        """
        {"available":true,"groups":[
          {"group":"gps","title":"GPS","facts":[
            {"name":"lat","label":"Latitude","selection":"gps/lat"},
            {"name":"hdop","label":"HDOP","selection":"gps/hdop"}]},
          {"group":"wind","title":"Wind","facts":[
            {"name":"speed","label":"Wind Speed","selection":"wind/speed"}]},
          {"group":"empty","title":"Empty","facts":[]}
        ]}
        """,
    )

    @Test
    fun `a group the vehicle reports nothing for is not offered`() {
        assertEquals(listOf("gps", "wind"), instrumentGroups(served).map { it.group })
    }

    @Test
    fun `a fact is asked for by the selection the core spells`() {
        val gps = instrumentGroups(served).first()
        assertEquals(
            "the head built this string itself until 7273d35a6 and got it wrong once already - a " +
                "bare lon resolves against the vehicle group and answers noSuchFact, which this " +
                "head drops silently, so the reading an operator picked never appeared",
            listOf("gps/lat", "gps/hdop"),
            gps.facts.map { it.path },
        )
        assertEquals("Latitude", gps.facts.first().label)
    }

    @Test
    fun `a fact the core did not spell a selection for is not offered`() {
        val unspelled = JSONObject(
            """{"available":true,"groups":[{"group":"gps","title":"GPS","facts":[
                 {"name":"lat","label":"Latitude"}]}]}""",
        )
        assertTrue(
            "a row that cannot be asked for is a row that silently does nothing when tapped",
            instrumentGroups(unspelled).isEmpty(),
        )
    }

    @Test
    fun `no vehicle offers nothing rather than an empty catalogue`() {
        assertTrue(instrumentGroups(JSONObject("""{"available":false,"groups":[]}""")).isEmpty())
        assertTrue(instrumentGroups(null).isEmpty())
    }

    @Test
    fun `choosing nothing draws nothing, and the core's own fallback is never reached`() {
        assertEquals("view.instruments(lat,hdop)", instrumentsPath(listOf("lat", "hdop")))
        assertTrue(showsInstruments(listOf("lat")))
        assertFalse(
            "instruments_view answers its OWN four defaults for an empty argument list, so a head " +
                "that stopped naming readings would get four back while the sheet drew them as " +
                "unchecked - the strip is gated here rather than by the path",
            showsInstruments(emptyList()),
        )
    }

    @Test
    fun `choosing toggles, and the row has a limit`() {
        assertEquals(listOf("a", "b"), withInstrument(listOf("a"), "b"))
        assertEquals(listOf("a"), withInstrument(listOf("a", "b"), "b"))

        val full = (1..MOST_INSTRUMENTS).map { "f$it" }
        assertEquals("a full row refuses another rather than dropping one silently", full, withInstrument(full, "extra"))
        assertEquals(
            "removing from a full row still works, or the limit would be a trap",
            full - "f1",
            withInstrument(full, "f1"),
        )
    }

    @Test
    fun `the note says where the operator stands`() {
        assertTrue(instrumentChoiceNote(emptyList()).contains("no readings"))
        assertTrue(instrumentChoiceNote(listOf("a", "b")).startsWith("2 of"))
        assertTrue(instrumentChoiceNote((1..MOST_INSTRUMENTS).map { "f$it" }).contains("Remove one"))
    }

    @Test
    fun `the readings the screen starts with are in the catalogue and match what is chosen`() {
        val own = vehicleOwnGroup(
            JSONObject(
                """{"kind":"object","facts":[
                     {"property":"altitudeRelative","shortDescription":"Alt (Rel)"},
                     {"property":"groundSpeed","shortDescription":"Ground Speed"},
                     {"property":"distanceToHome","shortDescription":"Distance to Home"},
                     {"property":"heading","shortDescription":"Heading"},
                     {"property":"rangeFinderDist","shortDescription":""}]}""",
            ),
        )!!
        assertEquals("Vehicle", own.title)
        assertTrue(
            "view.instrumentGroups enumerates the vehicle's CHILD groups and skips its own, so " +
                "without this the four defaults are absent from the sheet and cannot be unchecked",
            DEFAULT_INSTRUMENTS.all { name -> own.facts.any { it.path == name } },
        )
        assertEquals(
            "a top-level fact is asked for by bare name; only a child group's fact is qualified",
            "altitudeRelative",
            own.facts.first().path,
        )
        assertEquals(
            "five of the twenty-eight carry no description, and a blank row cannot be chosen from",
            "rangeFinderDist",
            own.facts.last().label,
        )
    }

    @Test
    fun `no vehicle contributes no group rather than an empty one`() {
        assertNull(vehicleOwnGroup(JSONObject("""{"kind":"null"}""")))
        assertNull(vehicleOwnGroup(JSONObject("""{"kind":"object","facts":[]}""")))
        assertNull(vehicleOwnGroup(null))
    }
}
