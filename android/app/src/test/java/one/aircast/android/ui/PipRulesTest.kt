package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class PipRulesTest {
    @Test
    fun `the 3D view is offered only while QGC's 3D viewer is enabled, or already showing`() {
        assertEquals(listOf(FlyView.Video, FlyView.Map), flyViewsOffered(false, FlyView.Video))
        assertEquals(FlyView.entries.toList(), flyViewsOffered(true, FlyView.Video))
        assertEquals(listOf(FlyView.Video, FlyView.Map), flyViewsOffered(false, FlyView.ThreeD))
        assertEquals(FlyView.entries.toList(), flyViewsOffered(null, FlyView.ThreeD))
        assertEquals(FlyView.Map, flyViewAllowed(false, FlyView.ThreeD))
        assertEquals(FlyView.ThreeD, flyViewAllowed(null, FlyView.ThreeD))
        assertEquals(FlyView.Video, flyViewAllowed(false, FlyView.Video))
    }
}
