package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class MapClickMenuTest {
    @Test
    fun `reads the offered actions in the order the core serves them`() {
        val actions = mapClickActions(
            JSONObject(
                """{"actions":[{"path":"vehicle.guidedModeGotoLocation","label":"Go to location","title":"Go To Location",""" +
                    """"message":"Move the vehicle to the specified location","confirm":true},""" +
                    """{"path":"vehicle.guidedModeROI","label":"ROI at location","title":"ROI","message":"m","confirm":false}]}""",
            ),
        )
        assertEquals(listOf("Go to location", "ROI at location"), actions.map { it.label })
        assertTrue(actions[0].confirm)
        assertFalse(actions[1].confirm)
        assertTrue(mapClickActions(null).isEmpty())
    }

    @Test
    fun `shows the point to six places as QGC does`() {
        assertEquals(listOf("Lat: 47.397000", "Lon: 8.545123"), coordinateLines(MapPoint(47.397, 8.5451234)))
    }
}
