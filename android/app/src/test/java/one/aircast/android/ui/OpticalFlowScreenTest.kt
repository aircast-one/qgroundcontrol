package one.aircast.android.ui

import org.junit.Assert.assertArrayEquals
import org.junit.Test

class OpticalFlowScreenTest {
    @Test
    fun `core rgba bytes become argb pixels`() {
        val rgba = byteArrayOf(0x11, 0x22, 0x33, 0xFF.toByte(), 0x80.toByte(), 0, 0x40, 0x7F)
        assertArrayEquals(intArrayOf(0xFF112233.toInt(), 0x7F800040), argbPixels(rgba, 2))
        assertArrayEquals("missing bytes read as transparent black", intArrayOf(0), argbPixels(byteArrayOf(), 1))
    }
}
