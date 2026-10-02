package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class PipRulesTest {
    @Test
    fun `the video picture-in-picture needs video and an expanded pip, like FlyView's item2 and IsPIPVisible`() {
        assertEquals(listOf(true, false, false), listOf(videoPipShown(true, true), videoPipShown(false, true), videoPipShown(true, false)))
    }
}
