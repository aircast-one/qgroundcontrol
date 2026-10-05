package one.aircast.map

import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Before
import org.junit.Test

class MapBridgeWatchTest {

    private val sent = mutableListOf<String>()

    @Before
    fun capture() {
        MapBridge.sendWatch = { sent += it }
    }

    @After
    fun drop() {
        MapBridge.forgetWatchesForTest()
    }

    @Test
    fun `a map leaving the screen keeps the paths another map still watches`() {
        MapBridge.watch("view.plan")
        MapBridge.watch("view.missionKinds")
        MapBridge.watch("view.plan")
        MapBridge.unwatch("view.plan")
        assertEquals(setOf("view.plan", "view.missionKinds"), MapBridge.watchedPathsForTest())
    }

    @Test
    fun `the last holder of a path unwatches it`() {
        MapBridge.watch("view.missionSummary")
        MapBridge.unwatch("view.missionSummary")
        assertEquals(emptySet<String>(), MapBridge.watchedPathsForTest())
        assertEquals("", sent.last())
    }

    @Test
    fun `a second holder of a path sends nothing new`() {
        MapBridge.watch("view.missionSummary")
        MapBridge.watch("view.missionSummary")
        assertEquals(listOf("view.missionSummary"), sent)
    }
}
