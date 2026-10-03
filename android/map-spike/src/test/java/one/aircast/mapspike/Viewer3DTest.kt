package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class Viewer3DTest {
    @Test
    fun `the core's buildings become extruded polygons centred on the file's bounds`() {
        val view = JSONObject("""{"class":"Viewer3D","available":true,"reason":null,"bounds":{"south":47.0,"west":8.0,"north":47.2,"east":8.2},
            "buildings":[{"outer":[[8.01,47.01],[8.02,47.01],[8.02,47.02],[8.01,47.01]],"inner":[],"height":6.0},{"outer":[[8.0,47.0]],"inner":[],"height":3.0}]}""")
        val scene = scene3d(view)
        assertEquals(1, scene.buildings.size)
        assertEquals(8.1, scene.centre!!.first, 1e-9)
        assertEquals(47.1, scene.centre!!.second, 1e-9)
        val feature = buildingFeatures(scene.buildings).features()!!.single()
        assertEquals(6.0, feature.getNumberProperty("height").toDouble(), 0.0)
    }

    @Test
    fun `an unavailable view carries the reason to show`() {
        val scene = scene3d(JSONObject("""{"available":false,"reason":"Turn on the 3D view in Settings.","buildings":[]}"""))
        assertEquals("Turn on the 3D view in Settings.", scene.reason)
    }
}
