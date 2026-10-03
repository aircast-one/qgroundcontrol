package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class RcHolderTest {

    @Test
    fun `a control releases every channel it sent, not the ones it is set to now`() {
        val queued = mutableListOf<() -> Unit>()
        val holder = RcHolder { queued += it }
        holder.hold(7, 1600)
        holder.hold(8, 1600)
        assertEquals("tilt moved from channel 7 to 8 mid-flight; both were sent, so both are held", setOf(7, 8), holder.holding())
        holder.release()
        assertEquals(emptySet<Int>(), holder.holding())
        assertEquals("two sends and one release, in that order on one queue", 3, queued.size)
    }

    @Test
    fun `an unassigned channel is never sent or held`() {
        val queued = mutableListOf<() -> Unit>()
        val holder = RcHolder { queued += it }
        holder.hold(0, 1600)
        holder.release()
        assertEquals(emptySet<Int>(), holder.holding())
        assertEquals(0, queued.size)
    }

    @Test
    fun `giving everything back forgets what was held without a second release`() {
        val queued = mutableListOf<() -> Unit>()
        val holder = RcHolder { queued += it }
        holder.hold(9, 2000)
        holder.forget()
        holder.release()
        assertEquals(1, queued.size)
    }
}
