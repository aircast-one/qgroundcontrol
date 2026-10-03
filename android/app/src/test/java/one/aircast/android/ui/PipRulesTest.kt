package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class PipRulesTest {
    @Test
    fun `the video picture-in-picture needs video and an expanded pip, like FlyView's item2 and IsPIPVisible`() {
        assertEquals(listOf(true, false, false), listOf(videoPipShown(true, true), videoPipShown(false, true), videoPipShown(true, false)))
    }

    @org.junit.Test
    fun `the 3D view is offered only while QGC's 3D viewer is enabled, or already showing`() {
        org.junit.Assert.assertEquals(listOf(FlyView.Video, FlyView.Map, FlyView.Simple), flyViewsOffered(false, FlyView.Video))
        org.junit.Assert.assertEquals(FlyView.entries.toList(), flyViewsOffered(true, FlyView.Video))
        org.junit.Assert.assertEquals(FlyView.entries.toList(), flyViewsOffered(false, FlyView.ThreeD))
    }
}
