package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Test

class PlanLayerSubtitleTest {
    @Test
    fun `layer subtitles read like PlanTreeView group headers`() {
        assertEquals("1 items", layerSubtitle(PlanLayer.Mission, 1, 0))
        assertEquals("0 items", layerSubtitle(PlanLayer.Mission, 0, 0))
        assertEquals("3 points", layerSubtitle(PlanLayer.Rally, 0, 3))
        assertEquals("", layerSubtitle(PlanLayer.Fence, 4, 2))
    }
}
