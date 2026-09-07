package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class VehicleTrackTest {
    @Test
    fun `a real fix is plottable`() {
        assertTrue(isPlottable(41.5, 44.5))
        assertTrue(isPlottable(-33.86, 151.2))
    }

    @Test
    fun `null island and missing fixes are refused`() {
        assertFalse(isPlottable(0.0, 0.0))
        assertFalse(isPlottable(Double.NaN, 44.5))
        assertFalse(isPlottable(41.5, Double.NaN))
    }

    @Test
    fun `out of range coordinates are refused`() {
        assertFalse(isPlottable(91.0, 0.5))
        assertFalse(isPlottable(-91.0, 0.5))
        assertFalse(isPlottable(10.0, 181.0))
        assertFalse(isPlottable(10.0, -181.0))
    }

    @Test
    fun `a track keeps the fixes it is given`() {
        val track = VehicleTrack()
        assertTrue(track.add(41.0, 44.0))
        assertTrue(track.add(41.1, 44.1))

        assertEquals(2, track.size)
        assertEquals(TrackPoint(41.0, 44.0), track.points().first())
        assertEquals(TrackPoint(41.1, 44.1), track.points().last())
    }

    @Test
    fun `a repeated fix does not extend the track`() {
        val track = VehicleTrack()
        assertTrue(track.add(41.0, 44.0))
        assertFalse(track.add(41.0, 44.0))

        assertEquals(1, track.size)
    }

    @Test
    fun `an unusable fix does not extend the track`() {
        val track = VehicleTrack()
        assertFalse(track.add(0.0, 0.0))
        assertFalse(track.add(Double.NaN, Double.NaN))

        assertEquals(0, track.size)
    }

    @Test
    fun `a long flight drops the oldest fixes`() {
        val track = VehicleTrack(limit = 3)
        (1..5).forEach { step -> track.add(41.0 + step * 0.01, 44.0) }

        assertEquals(3, track.size)
        assertEquals(TrackPoint(41.03, 44.0), track.points().first())
        assertEquals(TrackPoint(41.05, 44.0), track.points().last())
    }

    @Test
    fun `switching vehicle starts a new track instead of joining them`() {
        val track = VehicleTrack()
        track.add(41.0, 44.0)
        track.add(41.001, 44.001)

        track.add(-35.36, 149.16)

        assertEquals(1, track.size)
        assertEquals(TrackPoint(-35.36, 149.16), track.points().single())
    }

    @Test
    fun `normal flight movement never breaks the track`() {
        val track = VehicleTrack()
        (0..20).forEach { step -> track.add(41.0 + step * 0.001, 44.0 + step * 0.001) }

        assertEquals(21, track.size)
    }
}
