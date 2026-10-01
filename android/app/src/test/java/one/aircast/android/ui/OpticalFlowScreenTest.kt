package one.aircast.android.ui

import org.junit.Assert.assertArrayEquals
import org.junit.Test

class OpticalFlowScreenTest {
    @Test
    fun `a grey byte becomes an opaque pixel of that grey`() {
        assertArrayEquals(intArrayOf(0xFF000000.toInt(), 0xFFFFFFFF.toInt(), 0xFF808080.toInt()), greyPixels(byteArrayOf(0, 0xFF.toByte(), 0x80.toByte()), 3))
        assertArrayEquals("missing bytes read as black", intArrayOf(0xFF000000.toInt()), greyPixels(byteArrayOf(), 1))
    }
}
