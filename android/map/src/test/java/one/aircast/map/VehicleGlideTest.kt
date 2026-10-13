package one.aircast.map

import org.junit.Assert.assertEquals
import org.junit.Assert.assertSame
import org.junit.Test

class VehicleGlideTest {
    private val ms = 1_000_000L
    private val first = VehiclePose(41.0, 44.0, 350.0)
    private val second = VehiclePose(41.001, 44.0, 10.0)
    private val flying = Glide.settled(UNPLACED, 0).toward(first, 0).toward(second, 400 * ms)

    @Test
    fun `the marker moves at a steady speed from one fix to the next instead of jumping`() {
        assertEquals(first, Glide.settled(UNPLACED, 0).toward(first, 0).at(0))
        assertEquals(41.0005, flying.at(600 * ms).latitude, 1e-9)
        assertEquals(41.001, flying.at(800 * ms).latitude, 1e-9)
        assertEquals(41.001, flying.at(5_000 * ms).latitude, 1e-9)
    }

    @Test
    fun `the arrow turns the short way across north`() {
        assertEquals(360.0, flying.at(600 * ms).heading, 1e-9)
    }

    @Test
    fun `a heading update between fixes keeps the position on schedule`() {
        val turned = flying.toward(second.copy(heading = 20.0), 500 * ms)

        assertSame("an unchanged pose does not keep the map redrawing", flying, flying.toward(second, 500 * ms))
        assertEquals(flying.endNs, turned.endNs)
        assertEquals(flying.at(600 * ms).latitude, turned.at(600 * ms).latitude, 1e-9)
    }

    @Test
    fun `a fix after a long silence glides for at most a second`() {
        val resumed = flying.toward(VehiclePose(41.002, 44.0, 10.0), 60_000 * ms)

        assertEquals(1_000 * ms, resumed.endNs - resumed.startNs)
    }

    @Test
    fun `every vehicle in a fleet glides, not only the active one`() {
        val lead = VehicleChoice(id = 1, name = "Lead", state = "", link = "", contactLost = false, active = true, latitude = 41.0, longitude = 44.0, heading = 0.0)
        val wing = lead.copy(id = 2, name = "Wing", active = false, longitude = 45.0)
        val moved = listOf(lead.copy(latitude = 41.001), wing.copy(latitude = 41.001))
        val gliding = emptyMap<Int, Glide>().toward(listOf(lead, wing), 0).toward(moved, 400 * ms)

        assertEquals(listOf(41.0005, 41.0005), moved.glidedAt(gliding, 600 * ms).map { it.latitude })
        assertEquals(listOf(1), gliding.toward(moved.take(1), 500 * ms).keys.toList())
    }

    @Test
    fun `losing the position hides the marker at once`() {
        assertEquals(UNPLACED.latitude, flying.toward(UNPLACED, 500 * ms).at(500 * ms).latitude, 0.0)
    }
}
