package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Test

class TerrainCoverageTest {
    private fun at(distance: Double, terrain: Double?) =
        ProfilePoint(distance, terrain, 100.0)

    @Test
    fun `terrain under the whole route is full coverage`() {
        assertEquals(
            1.0,
            TerrainProfile(listOf(at(0.0, 400.0), at(500.0, 410.0), at(1000.0, 420.0))).terrainCoverage,
            1e-9,
        )
    }

    @Test
    fun `a leg with no ground under it does not count`() {
        assertEquals(
            0.5,
            TerrainProfile(listOf(at(0.0, 400.0), at(500.0, 410.0), at(1000.0, null))).terrainCoverage,
            1e-9,
        )
    }

    @Test
    fun `ground at one end only is measured by distance not by sample count`() {
        val dense = (0..9).map { at(it * 10.0, 400.0) }
        val bare = listOf(at(1000.0, null), at(2000.0, null))
        assertEquals(0.045, TerrainProfile(dense + bare).terrainCoverage, 1e-9)
    }

    @Test
    fun `no terrain anywhere is no coverage`() {
        assertEquals(
            0.0,
            TerrainProfile(listOf(at(0.0, null), at(1000.0, null))).terrainCoverage,
            1e-9,
        )
    }

    @Test
    fun `a route of no length has no coverage rather than dividing by zero`() {
        assertEquals(0.0, TerrainProfile(emptyList()).terrainCoverage, 1e-9)
    }
}
