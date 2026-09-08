package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Test

class SegmentRotationTest {
    @Test
    fun `an item whose key is unchanged is kept`() {
        assertEquals(
            setOf(1, 3),
            stillTheSameItems(mapOf(1 to "a", 3 to "b"), mapOf(1 to "a", 3 to "b")),
        )
    }

    @Test
    fun `a plan of the same length over different ground keeps nothing`() {
        assertEquals(
            emptySet<Int>(),
            stillTheSameItems(mapOf(1 to "a", 3 to "b"), mapOf(1 to "c", 3 to "d")),
        )
    }

    @Test
    fun `an item that is gone is not kept even if some other index matches`() {
        assertEquals(
            setOf(1),
            stillTheSameItems(mapOf(1 to "a", 3 to "b"), mapOf(1 to "a")),
        )
    }

    @Test
    fun `an item never seen before is not claimed as unchanged`() {
        assertEquals(emptySet<Int>(), stillTheSameItems(emptyMap(), mapOf(1 to "a")))
    }

    @Test
    fun `four are read a poll`() {
        assertEquals(setOf(1, 2, 3, 4), readThisPoll(listOf(1, 2, 3, 4, 5, 6), 0))
    }

    @Test
    fun `the next poll takes the ones the last poll did not`() {
        assertEquals(setOf(5, 6, 1, 2), readThisPoll(listOf(1, 2, 3, 4, 5, 6), 4))
    }

    @Test
    fun `fewer items than a poll reads means all of them`() {
        assertEquals(setOf(1, 2), readThisPoll(listOf(1, 2), 0))
    }

    @Test
    fun `a plan with no surveys reads nothing`() {
        assertEquals(emptySet<Int>(), readThisPoll(emptyList(), 7))
    }

    @Test
    fun `the rotation keeps working once the counter has run past the list`() {
        assertEquals(setOf(3, 4, 5, 6), readThisPoll(listOf(1, 2, 3, 4, 5, 6), 14))
    }
}
