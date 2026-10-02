package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ProximityRadarMapTest {
    private val centre = TrackPoint(47.4, 8.5)

    @Test
    fun `hidden radar reads nothing and sectors without a distance are dropped`() {
        assertNull(radarReading(JSONObject("""{"shown":false}""")))
        val reading = radarReading(JSONObject("""{"shown":true,"maxMeters":40,"sectors":[{"bearing":0,"meters":10},{"bearing":45,"meters":null}]}"""))
        assertEquals(RadarReading(40.0, listOf(0.0 to 10.0)), reading)
        assertNull(radarReading(JSONObject("""{"shown":true,"maxMeters":null,"sectors":[]}"""))?.maxMeters)
    }

    @Test
    fun `limit circle closes and each arc spans its sector turned by heading`() {
        val lines = radarLines(centre, 90.0, RadarReading(40.0, listOf(0.0 to 10.0)))
        assertEquals(listOf("#FFFFFF", "#FF0000"), lines.map { it.first })
        assertEquals(lines[0].second.first(), lines[0].second.last())
        val arc = lines[1].second
        arc.forEach { assertEquals(10.0, metresBetween(centre, it), 0.01) }
        assertEquals(pointAt(centre, 10.0, 67.5).latitude, arc.first().latitude, 1e-9)
        assertEquals(pointAt(centre, 10.0, 112.5).longitude, arc.last().longitude, 1e-9)
    }

    @Test
    fun `no max distance draws only the arcs`() {
        assertEquals(1, radarLines(centre, Double.NaN, RadarReading(null, listOf(180.0 to 5.0))).size)
    }
}

class FleetRadarTest {

    private val reading = RadarReading(40.0, listOf(0.0 to 10.0))

    @Test
    fun `every vehicle with sensors gets its own radar, the active one at its live position`() {
        val fleet = listOf(
            VehicleChoice(id = 1, name = "A", state = "", link = "", contactLost = false, active = true, latitude = 1.0, longitude = 1.0, radar = reading),
            VehicleChoice(id = 2, name = "B", state = "", link = "", contactLost = false, active = false, latitude = 47.0, longitude = 8.0, heading = 90.0, radar = reading),
            VehicleChoice(id = 3, name = "C", state = "", link = "", contactLost = false, active = false, latitude = 48.0, longitude = 8.0),
        )
        val placed = placedRadars(fleet, TrackPoint(47.5, 8.5), 10.0)
        assertEquals(listOf(TrackPoint(47.5, 8.5), TrackPoint(47.0, 8.0)), placed.map { it.at })
        assertEquals(listOf(10.0, 90.0), placed.map { it.heading })
    }
}
