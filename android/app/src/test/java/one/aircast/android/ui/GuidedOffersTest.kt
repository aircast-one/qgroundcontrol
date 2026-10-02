package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class GuidedOffersTest {
    @Test
    fun `the rtl offer carries the core's Smart RTL option`() {
        val offers = guidedOffers(JSONObject("""{"actions":[{"id":"rtl","offer":"ready","option":"Smart RTL"},{"id":"land","offer":"ready","option":""}]}"""))
        assertEquals("Smart RTL", offers.getValue("rtl").option)
        assertEquals("", offers.getValue("land").option)
    }

    @Test
    fun `the multi-vehicle panel is on unless the setting turns it off`() {
        assertEquals(listOf(true, true, false), listOf(null, JSONObject("""{"value":true}"""), JSONObject("""{"value":false}""")).map { multiVehiclePanelEnabled(it) })
    }
}
