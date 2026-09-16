package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ProfileOffsetsTest {

    private fun at(distance: Double, terrain: Double?, planned: Double) =
        ProfilePoint(distance = distance, terrain = terrain, planned = planned)

    private fun profile(vararg points: ProfilePoint) = TerrainProfile(points = points.toList())

    @Test
    fun `flat ground divides by the floor rather than by zero`() {
        val flat = profile(at(0.0, null, 100.0), at(50.0, null, 100.0), at(100.0, null, 100.0))
        val offsets = profileOffsets(flat, 200f, 80f) { it.planned }

        assertTrue(
            "span is (highest - lowest) coerced to MIN_SPAN_METRES, so level ground gives 1.0 " +
                "rather than 0.0. Without that floor every y here is 0.0/0.0 = NaN and the " +
                "profile draws nothing, with no error to notice - the guard lives in the producer, " +
                "a file away from the division that depends on it",
            offsets.isNotEmpty() && offsets.all { it.x.isFinite() && it.y.isFinite() },
        )
    }

    @Test
    fun `a profile spans the full width and sits inside the height`() {
        val slope = profile(at(0.0, 10.0, 10.0), at(100.0, 110.0, 110.0))
        val offsets = profileOffsets(slope, 200f, 80f) { it.terrain }

        assertEquals(2, offsets.size)
        assertEquals("the first point is the left edge", 0f, offsets.first().x, 0.001f)
        assertEquals("the last point is the right edge", 200f, offsets.last().x, 0.001f)
        assertEquals("the lowest point sits on the floor", 80f, offsets.first().y, 0.001f)
        assertEquals("the highest point sits on the ceiling", 0f, offsets.last().y, 0.001f)
    }

    @Test
    fun `nothing is drawn without a width, a height or any distance`() {
        val slope = profile(at(0.0, 10.0, 10.0), at(100.0, 110.0, 110.0))
        assertTrue(profileOffsets(slope, 0f, 80f) { it.terrain }.isEmpty())
        assertTrue(profileOffsets(slope, 200f, 0f) { it.terrain }.isEmpty())
        assertTrue(
            "a plan with no length is the other division here, and that one is guarded at the " +
                "use site rather than in the producer",
            profileOffsets(profile(at(0.0, 10.0, 10.0)), 200f, 80f) { it.terrain }.isEmpty(),
        )
    }

    @Test
    fun `a point the sensor never reported is skipped rather than plotted at zero`() {
        val gap = profile(at(0.0, 10.0, 10.0), at(50.0, null, 20.0), at(100.0, 30.0, 30.0))
        assertEquals(
            "terrain is null where nothing was sampled; plotting it as 0 would draw a canyon " +
                "that is not there",
            2,
            profileOffsets(gap, 200f, 80f) { it.terrain }.size,
        )
    }
}
