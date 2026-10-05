package one.aircast.map

import org.junit.Assert.assertEquals
import org.junit.Test

class ItemCountTextTest {
    @Test
    fun singularForOne() = assertEquals("1 item", itemCountText(1))

    @Test
    fun pluralOtherwise() {
        assertEquals("0 items", itemCountText(0))
        assertEquals("6 items", itemCountText(6))
    }
}
