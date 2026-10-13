package one.aircast.android.ui

import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class StickInsetsTest {
    private val pad = 172.dp

    private fun gaps(letterbox: Letterbox, controls: Dp, fullScreen: Boolean = false): Pair<Dp, Dp> {
        val insets = stickInsets(pad, fullScreen, letterbox, 0.dp, 0.dp, controls)
        val trailing = if (fullScreen) 0.dp else controls + insets.trailingGap
        return letterbox.start + insets.rowStart to letterbox.end + insets.rowEnd + trailing
    }

    @Test
    fun `both sticks sit the same distance from their glass edge when the camera column is the wider side`() {
        assertEquals(164.dp to 164.dp, gaps(Letterbox(52.dp, 0.dp), controls = 144.dp))
    }

    @Test
    fun `the action rail sets the inset when the camera column is narrow, and the right stick matches it`() {
        assertEquals(124.dp to 124.dp, gaps(Letterbox(52.dp, 0.dp), controls = 40.dp))
    }

    @Test
    fun `full screen mirrors the sticks across a letterboxed cutout band`() {
        assertEquals(64.dp to 64.dp, gaps(Letterbox(52.dp, 0.dp), controls = 144.dp, fullScreen = true))
    }

    @Test
    fun `without sticks the overlays keep the rail clearance and the map keeps its ornaments`() {
        val insets = stickInsets(null, false, Letterbox(0.dp, 0.dp), 0.dp, 0.dp, 0.dp)
        assertEquals(72.dp, insets.overlaysStart)
        assertEquals(12.dp, insets.overlaysEnd)
        assertNull(insets.ornamentEnd)
    }
}
