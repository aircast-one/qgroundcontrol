package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class Viewer3DTest {
    @Test
    fun `the core's buildings become extruded polygons centred on the file's bounds`() {
        val view = JSONObject("""{"class":"Viewer3D","available":true,"reason":null,"bounds":{"south":47.0,"west":8.0,"north":47.2,"east":8.2},
            "buildings":[{"outer":[[[8.01,47.01],[8.02,47.01],[8.02,47.02],[8.01,47.01]]],"inner":[],"height":6.0},{"outer":[[[8.0,47.0]]],"inner":[],"height":3.0}]}""")
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

    @Test
    fun `outer rings wind counter-clockwise and holes clockwise so MapLibre keeps courtyards as holes`() {
        val square = listOf(0.0 to 0.0, 0.0 to 1.0, 1.0 to 1.0, 1.0 to 0.0)
        assertEquals(true, signedArea(wound(square, counterClockwise = true)) > 0)
        assertEquals(true, signedArea(wound(square, counterClockwise = false)) < 0)
        assertEquals(wound(square, true).first(), wound(square, true).last())
    }

    @Test
    fun `a climbing segment floats as stepped slabs at its altitude`() {
        val pieces = ribbon(Point3D(8.0, 47.0, 20.0), Point3D(8.0, 47.0003, 40.0), "orange")
        assertEquals(5, pieces.size)
        assertEquals(true, pieces.first().base < pieces.last().base)
        assertEquals(22.0, (pieces.first().base + pieces.first().top) / 2, 1e-9)
    }

    @Test
    fun `markers float at their altitude and the vehicle comes from its own view`() {
        val view = JSONObject("""{"markers":[{"at":[8.0,47.0,30.0],"name":"W","colour":"black"}],"segments":[]}""")
        assertEquals(listOf(28.5), pathSlabs(view).map { it.base })
        val one = vehicleSlabs(JSONObject("""{"vehicles":[{"at":[8.0,47.0,12.0],"heading":0.0}]}"""))
        assertEquals(true, one.isNotEmpty() && one.all { it.base in 11.0..13.0 })
        assertEquals(2 * one.size, vehicleSlabs(JSONObject("""{"vehicles":[{"at":[8.0,47.0,12.0],"heading":0.0},{"at":[8.001,47.0,30.0],"heading":90.0}]}""")).size)
        assertEquals(emptyList<Slab>(), vehicleSlabs(JSONObject("""{"vehicles":[]}""")))
    }

    @Test
    fun `the quad frame's red front arms point along the heading`() {
        val at = Point3D(8.0, 47.0, 20.0)
        val centre = { slabs: List<Slab> -> slabs.flatMap { it.corners }.map { it.first }.average() }
        val frame = quadFrame(at, 90.0)
        assertEquals(true, centre(frame.filter { it.colour == "#E53935" }) > at.lon)
        assertEquals(true, centre(frame.filter { it.colour == "#ECEFF1" }) < at.lon)
        val north = quadFrame(at, 0.0).filter { it.colour == "#E53935" }.flatMap { it.corners }.map { it.second }.average()
        assertEquals(true, north > at.lat)
    }

    @Test
    fun `a straight climb is one column from the lower to the higher altitude`() {
        val column = ribbon(Point3D(8.0, 47.0, 10.0), Point3D(8.0, 47.0, 40.0), "orange")
        assertEquals(1, column.size)
        assertEquals(9.5 to 40.5, column.single().base to column.single().top)
    }

    @Test
    fun `a steep climb leaves no gaps and a long leg stays capped`() {
        val steep = ribbon(Point3D(8.0, 47.0, 0.0), Point3D(8.0, 47.0003, 20.0), "orange")
        assertEquals(true, steep.zipWithNext().all { (a, b) -> b.base <= a.top + 1e-9 })
        assertEquals(64, ribbon(Point3D(8.0, 47.0, 10.0), Point3D(8.0, 48.0, 10.0), "orange").size)
    }

    @Test
    fun `every marker carries its label three metres above it like Viewer3DVehicleItems`() {
        val labels = pathLabels(JSONObject("""{"markers":[{"at":[8.0,47.0,30.0],"name":"W","label":"3","colour":"black"},{"at":[8.1,47.1,0.0],"name":"","label":"","colour":"black"}]}"""))
        assertEquals(listOf(Label3D(Point3D(8.0, 47.0, 33.0), "3")), labels)
    }

    @Test
    fun `a label rises on screen by its height foreshortened by the camera tilt`() {
        assertEquals(100f to 200f, lifted(100f to 200f, 2.0, 0.0, 50.0))
        assertEquals(100f to 150f, lifted(100f to 200f, 2.0, 30.0, 50.0))
        assertEquals(100f to 200f, lifted(100f to 200f, 2.0, 30.0, -5.0))
    }
}
