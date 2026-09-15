package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class InstrumentChoiceTest {

    private val served = JSONObject(
        """
        {"available":true,"groups":[
          {"group":"gps","title":"GPS","facts":[
            {"name":"lat","label":"Latitude"},{"name":"hdop","label":"HDOP"}]},
          {"group":"wind","title":"Wind","facts":[{"name":"speed","label":"Wind Speed"}]},
          {"group":"empty","title":"Empty","facts":[]}
        ]}
        """,
    )

    @Test
    fun `a group the vehicle reports nothing for is not offered`() {
        assertEquals(listOf("gps", "wind"), instrumentGroups(served).map { it.group })
    }

    @Test
    fun `a fact is asked for by its group and name together`() {
        val gps = instrumentGroups(served).first()
        assertEquals(
            "view.instruments answers noSuchFact for a bare lon and the head drops it silently, " +
                "so an unqualified name reads as a reading that simply never appears",
            listOf("gps.lat", "gps.hdop"),
            gps.facts.map { it.path },
        )
        assertEquals("Latitude", gps.facts.first().label)
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
}
