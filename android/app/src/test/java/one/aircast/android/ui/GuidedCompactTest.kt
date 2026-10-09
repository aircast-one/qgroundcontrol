package one.aircast.android.ui

import androidx.compose.ui.unit.dp
import org.junit.Assert.assertEquals
import org.junit.Test

class GuidedCompactTest {
    @Test
    fun `a landscape phone folds the panel so the hold button never pushes the value out of sight`() {
        assertEquals(listOf(true, false, false), listOf(guidedCompact(portrait = false, 344.dp), guidedCompact(portrait = false, 600.dp), guidedCompact(portrait = true, 344.dp)))
    }

    @Test
    fun `the folded panel carries the range beside the label instead of a line of its own`() {
        assertEquals("Height above launch · 3.0 to 100.0 m", guidedLabel("Height above launch", "3.0 to 100.0 m", compact = true))
        assertEquals("Height above launch", guidedLabel("Height above launch", "3.0 to 100.0 m", compact = false))
        assertEquals("Height above launch", guidedLabel("Height above launch", null, compact = true))
    }
}
