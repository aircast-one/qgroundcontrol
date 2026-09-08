package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class PlanSummaryTest {
    private fun item(index: Int = 0, altitude: Double = Double.NaN) =
        MissionItem(index, index + 1, 41.0, 44.0, "Waypoint", false, altitude)

    private fun summary(
        items: List<MissionItem> = emptyList(),
        circles: List<FenceCircle> = emptyList(),
        distance: Double = Double.NaN,
        seconds: Double = Double.NaN,
        selected: MapHit? = null,
        itemCount: Int = items.size,
        shape: List<String> = emptyList(),
    ) = planSummary(
        itemCount, shape, items, emptyList(), circles, emptyList(), emptyList(),
        distance, seconds, selected,
    )

    @Test
    fun `what is in the plan but not on the map is named`() {
        assertTrue(
            summary(items = listOf(item()), itemCount = 3, shape = listOf("takeoff", "RTL"))
                .startsWith("3 items (takeoff, RTL)"),
        )
    }

    @Test
    fun `items that cannot be drawn are still in the plan`() {
        assertTrue(summary(items = listOf(item()), itemCount = 3).startsWith("3 items"))
    }

    @Test
    fun `an empty plan says how to start one`() {
        assertEquals("Empty plan · long press to add", summary())
    }

    @Test
    fun `only what the plan actually holds is listed`() {
        assertEquals("2 items", summary(items = listOf(item(0), item(1))))
    }

    @Test
    fun `counts read as singular when there is one`() {
        assertEquals("1 item", summary(items = listOf(item())))
    }

    @Test
    fun `cost is appended when the controller has worked it out`() {
        val text = summary(items = listOf(item()), distance = 2000.0, seconds = 240.0)

        assertEquals("1 item · 2.00 km · 4:00", text)
    }

    @Test
    fun `a selected waypoint shows its altitude`() {
        val text = summary(
            items = listOf(item(index = 3, altitude = 75.0)),
            selected = MapHit.Waypoint(3),
        )

        assertTrue(text.endsWith("#3 at 75 m"))
    }

    @Test
    fun `a selected circle shows its radius from either handle`() {
        val circles = listOf(FenceCircle(1, true, TrackPoint(41.0, 44.0), 136.0))

        assertTrue(summary(circles = circles, selected = MapHit.Circle(1)).endsWith("circle 136 m"))
        assertTrue(summary(circles = circles, selected = MapHit.CircleCentre(1)).endsWith("circle 136 m"))
    }

    @Test
    fun `a selection with nothing to say adds nothing`() {
        assertEquals("1 item", summary(items = listOf(item()), selected = MapHit.Waypoint(0)))
    }
}
