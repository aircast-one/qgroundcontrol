package one.aircast.android.bridge

import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Test

class QgcWatchTest {
    private val sent = mutableListOf<String>()

    @After
    fun restore() {
        Qgc.sendWatch = { }
        Qgc.forgetWatchesForTest()
    }

    private fun succeeding() {
        Qgc.sendWatch = { sent.add(it) }
    }

    private fun failing(reason: Throwable) {
        Qgc.sendWatch = { throw reason }
    }

    @Test
    fun `a path registered before the natives exist is retried, not lost forever`() {
        Qgc.forgetWatchesForTest()
        failing(UnsatisfiedLinkError("No implementation found for QGCBridge.watch"))
        Qgc.watch(listOf("vehicle.armed"))

        succeeding()
        Qgc.watch(listOf("vehicle.armed"))

        assertEquals(listOf("vehicle.armed"), sent)
    }

    @Test
    fun `a path that registered once is not sent again`() {
        Qgc.forgetWatchesForTest()
        succeeding()
        Qgc.watch(listOf("vehicle.armed"))
        Qgc.watch(listOf("vehicle.armed"))

        assertEquals(1, sent.size)
    }

    @Test
    fun `a later path is registered alongside the earlier ones`() {
        Qgc.forgetWatchesForTest()
        succeeding()
        Qgc.watch(listOf("a"))
        Qgc.watch(listOf("b"))

        assertEquals(listOf("a", "a,b"), sent)
    }

    @Test
    fun `a failure does not discard paths that already registered`() {
        Qgc.forgetWatchesForTest()
        succeeding()
        Qgc.watch(listOf("a"))

        failing(IllegalStateException("bridge went away"))
        Qgc.watch(listOf("b"))

        succeeding()
        Qgc.watch(listOf("b"))

        assertEquals(listOf("a", "a,b"), sent)
    }

    @Test
    fun `a path is only dropped once its last holder releases it`() {
        Qgc.forgetWatchesForTest()
        succeeding()
        Qgc.watch(listOf("a"))
        Qgc.watch(listOf("a"))

        Qgc.unwatch(listOf("a"))
        assertEquals(setOf("a"), Qgc.watchedPathsForTest())

        Qgc.unwatch(listOf("a"))
        assertEquals(emptySet<String>(), Qgc.watchedPathsForTest())
        assertEquals(listOf("a", ""), sent)
    }

    @Test
    fun `releasing one path leaves the others registered`() {
        Qgc.forgetWatchesForTest()
        succeeding()
        Qgc.watch(listOf("a"))
        Qgc.watch(listOf("b"))
        Qgc.unwatch(listOf("a"))

        assertEquals(setOf("b"), Qgc.watchedPathsForTest())
        assertEquals(listOf("a", "a,b", "b"), sent)
    }

    @Test
    fun `releasing a path that was never registered changes nothing`() {
        Qgc.forgetWatchesForTest()
        succeeding()
        Qgc.watch(listOf("a"))
        Qgc.unwatch(listOf("never"))

        assertEquals(setOf("a"), Qgc.watchedPathsForTest())
        assertEquals(listOf("a"), sent)
    }

    @Test
    fun `a failed release keeps the path registered rather than silently dropping it`() {
        Qgc.forgetWatchesForTest()
        succeeding()
        Qgc.watch(listOf("a"))

        failing(IllegalStateException("bridge went away"))
        Qgc.unwatch(listOf("a"))

        assertEquals(setOf("a"), Qgc.watchedPathsForTest())

        succeeding()
        Qgc.unwatch(listOf("a"))
        assertEquals(listOf("a", ""), sent)
    }
}
