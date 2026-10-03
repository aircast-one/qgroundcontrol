package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class FenceListTest {
    @Test
    fun `a row selects its shape and a deleted rally point hands selection to its neighbour`() {
        val polygon = FenceRow(1, false, "Keep-out polygon 2", "", inclusion = false)
        val circle = FenceRow(0, true, "Keep-in circle 1", "")
        assertEquals(MapHit.FenceVertex(1, 0), fenceRowHit(polygon))
        assertEquals(MapHit.Circle(0), fenceRowHit(circle))
        assertTrue(rowSelected(polygon, MapHit.FenceVertex(1, 3)))
        assertEquals("RallyPointController selects the next point", MapHit.Rally(1), rallyAfterRemove(1, 3))
        assertEquals("or the new last one", MapHit.Rally(1), rallyAfterRemove(2, 3))
        assertNull(rallyAfterRemove(0, 1))
    }
}
