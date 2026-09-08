package one.aircast.mapspike

import androidx.compose.ui.geometry.Offset
import org.junit.Assert.assertEquals
import org.junit.Test

class GroundOutlineTest {
    @Test
    fun `the fill closes under the terrain it has, not across the whole chart`() {
        assertEquals(
            listOf(Offset(10f, 5f), Offset(40f, 8f), Offset(40f, 100f), Offset(10f, 100f)),
            groundOutline(listOf(Offset(10f, 5f), Offset(40f, 8f)), 100f),
        )
    }

    @Test
    fun `terrain covering the whole width still closes at the corners`() {
        assertEquals(
            listOf(Offset(0f, 5f), Offset(200f, 8f), Offset(200f, 100f), Offset(0f, 100f)),
            groundOutline(listOf(Offset(0f, 5f), Offset(200f, 8f)), 100f),
        )
    }

    @Test
    fun `a single sample is not a shape`() {
        assertEquals(emptyList<Offset>(), groundOutline(listOf(Offset(10f, 5f)), 100f))
    }

    @Test
    fun `no terrain draws no ground`() {
        assertEquals(emptyList<Offset>(), groundOutline(emptyList(), 100f))
    }
}
