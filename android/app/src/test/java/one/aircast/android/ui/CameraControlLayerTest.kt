package one.aircast.android.ui

import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntRect
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.LayoutDirection
import org.junit.Assert.assertEquals
import org.junit.Test

class CameraControlLayerTest {
    private val window = IntSize(1000, 500)
    private val popup = IntSize(100, 200)

    private fun placed(anchor: IntRect, direction: LayoutDirection = LayoutDirection.Ltr) = BesideAnchor.calculatePosition(anchor, window, direction, popup)

    @Test
    fun `the zoom slider opens beside the chip, centred on it`() {
        assertEquals(IntOffset(500, 150), placed(IntRect(600, 230, 680, 270)))
    }

    @Test
    fun `the zoom slider stays on screen next to an edge`() {
        assertEquals("a chip at the left edge keeps the slider inside the window", IntOffset(0, 150), placed(IntRect(40, 230, 120, 270)))
        assertEquals("a chip near the top keeps the slider below the window edge", IntOffset(500, 0), placed(IntRect(600, 10, 680, 50)))
        assertEquals("a chip near the bottom keeps the slider above the window edge", IntOffset(500, 300), placed(IntRect(600, 460, 680, 500)))
    }

    @Test
    fun `right to left the zoom slider opens on the chip's other side`() {
        assertEquals(IntOffset(680, 150), placed(IntRect(600, 230, 680, 270), LayoutDirection.Rtl))
    }
}
