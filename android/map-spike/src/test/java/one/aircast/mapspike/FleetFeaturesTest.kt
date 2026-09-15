package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class FleetFeaturesTest {

    private fun fleet(json: String) = vehicleChoices(JSONObject(json)).choices

    private val two = fleet(
        """
        {"ambiguous":true,"vehicles":[
          {"id":1,"name":"Quadrotor 1","active":true,"contactLost":false,
           "coordinate":{"latitude":41.7,"longitude":-74.0}},
          {"id":2,"name":"Quadrotor 2","active":false,"contactLost":true,
           "coordinate":{"latitude":41.8,"longitude":-74.1}}
        ]}
        """,
    )

    @Test
    fun `every aircraft is drawn, not only the one being flown`() {
        val drawn = fleetFeatures(two, activeHeading = 90.0).features()!!
        assertEquals(2, drawn.size)
        assertEquals(-74.0, drawn[0].geometry().let { (it as org.maplibre.geojson.Point).longitude() }, 1e-9)
        assertEquals(-74.1, drawn[1].geometry().let { (it as org.maplibre.geojson.Point).longitude() }, 1e-9)
    }

    @Test
    fun `only the aircraft being flown carries a heading`() {
        val drawn = fleetFeatures(two, activeHeading = 90.0).features()!!
        assertTrue(
            "view.vehicles serves no heading, so the others would be arrows pointing north - a " +
                "direction nobody reported",
            drawn[0].hasProperty(HEADING_PROPERTY),
        )
        assertFalse(drawn[1].hasProperty(HEADING_PROPERTY))
    }

    @Test
    fun `each aircraft carries its own silence and its own place in the list`() {
        val drawn = fleetFeatures(two, activeHeading = Double.NaN).features()!!
        assertFalse(drawn[0].getBooleanProperty(STALE_PROPERTY))
        assertTrue("the second stopped answering and the map should not draw it as live", drawn[1].getBooleanProperty(STALE_PROPERTY))
        assertTrue(drawn[0].getBooleanProperty(ACTIVE_PROPERTY))
        assertFalse(drawn[1].getBooleanProperty(ACTIVE_PROPERTY))
    }

    @Test
    fun `an aircraft that has reported no position is left off rather than drawn at zero`() {
        val unplaced = fleet(
            """{"vehicles":[
                 {"id":1,"name":"A","active":true,"coordinate":{"latitude":41.7,"longitude":-74.0}},
                 {"id":2,"name":"B","active":false,"coordinate":null},
                 {"id":3,"name":"C","active":false,"coordinate":{"latitude":0,"longitude":0}}]}""",
        )
        assertEquals(
            "null island is where an unset coordinate lands, and a marker there is a lie about " +
                "an aircraft rather than an absence",
            1,
            fleetFeatures(unplaced, activeHeading = 0.0).features()!!.size,
        )
    }

    @Test
    fun `no fleet draws nothing rather than a marker at nowhere`() {
        assertEquals(0, fleetFeatures(emptyList(), activeHeading = 0.0).features()!!.size)
    }
}
