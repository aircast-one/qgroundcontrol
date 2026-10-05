package one.aircast.map

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Test

class MidpointTest {

    private val ring = EditableShape(
        path = "plan.geoFenceController.polygons.0",
        midpoints = listOf(
            TrackPoint(41.700, 44.825), TrackPoint(41.705, 44.830),
            TrackPoint(41.710, 44.825), TrackPoint(41.705, 44.820),
        ),
        splitInvokable = "splitPolygonSegment",
        canRemoveVertex = true,
    )

    private val square = FencePolygon(
        index = 0,
        inclusion = true,
        vertices = listOf(
            TrackPoint(41.70, 44.82), TrackPoint(41.70, 44.83),
            TrackPoint(41.71, 44.83), TrackPoint(41.71, 44.82),
        ),
        editable = ring,
    )

    private val corridor = EditableShape(
        path = "plan.missionController.visualItems.2.corridorPolyline",
        midpoints = listOf(TrackPoint(41.700, 44.825), TrackPoint(41.705, 44.830)),
        splitInvokable = "splitSegment",
        canRemoveVertex = true,
    )

    @Test
    fun `every segment offers somewhere to add a corner`() {
        val features = midpointFeatures(listOf(ring)).features()!!

        assertEquals(4, features.size)
        assertEquals(
            listOf(0, 1, 2, 3),
            features.map { it.getNumberProperty(VERTEX_INDEX_PROPERTY).toInt() },
        )
    }

    @Test
    fun `a survey edge is split by its own invokable, not the fence one`() {
        val features = midpointFeatures(listOf(ring, corridor)).features()!!

        assertEquals(6, features.size)
        assertEquals(
            listOf("splitPolygonSegment", "splitPolygonSegment", "splitPolygonSegment", "splitPolygonSegment", "splitSegment", "splitSegment"),
            features.map { it.getStringProperty(SPLIT_INVOKABLE_PROPERTY) },
        )
        assertEquals(
            listOf(ring.path, corridor.path),
            features.map { it.getStringProperty(SHAPE_PATH_PROPERTY) }.distinct(),
        )
    }

    @Test
    fun `a shape the core gave no midpoints for offers none`() {
        assertEquals(0, midpointFeatures(listOf(ring.copy(midpoints = emptyList()))).features()!!.size)
        assertEquals(0, midpointFeatures(listOf(null)).features()!!.size)
        assertEquals(0, midpointFeatures(emptyList()).features()!!.size)
    }

    @Test
    fun `a shape the core named no invokable for is not offered`() {
        assertEquals(0, midpointFeatures(listOf(ring.copy(splitInvokable = ""))).features()!!.size)
    }

    @Test
    fun `a midpoint is a button, so it never writes a position`() {
        assertFalse(writeMove(MapHit.Midpoint(ring.path, ring.splitInvokable, 2), 41.0, 44.0, emptyList(), emptyList()))
    }

    @Test
    fun `a midpoint is never a lasting selection`() {
        assertFalse(
            selectionSurvives(
                MapHit.Midpoint(ring.path, ring.splitInvokable, 2),
                emptyList(), listOf(square), emptyList(), emptyList(), emptyList(),
            ),
        )
    }

    @Test
    fun `the leg into the selected item splits at its middle`() {
        val item = { index: Int, lat: Double, lon: Double -> MissionItem(index, index, lat, lon, "Waypoint", false, 50.0) }
        val items = listOf(item(1, 47.0, 8.0), item(2, 47.2, 8.4))
        val split = legSplit(items, 2) ?: error("split")
        org.junit.Assert.assertEquals(metresBetween(TrackPoint(47.0, 8.0), split), metresBetween(split, TrackPoint(47.2, 8.4)), 0.5)
        org.junit.Assert.assertEquals(47.1, split.latitude, 0.01)
        org.junit.Assert.assertEquals(90.0, azimuthBetween(TrackPoint(0.0, 0.0), TrackPoint(0.0, 1.0)), 1e-9)
        org.junit.Assert.assertNull(legSplit(items, 1))
        org.junit.Assert.assertNull(legSplit(items, null))
    }

    @Test
    fun `the split skips home and standalone items and starts at a pattern's exit, as MissionController does`() {
        val item = { index: Int, lat: Double, routed: Boolean, exit: TrackPoint? -> MissionItem(index, index, lat, 8.0, "Waypoint", false, 50.0, exit = exit, routed = routed) }
        val home = item(0, 46.0, true, null)
        org.junit.Assert.assertNull("the leg from home is not split", legSplit(listOf(home, item(1, 47.0, true, null)), 1))
        val roi = item(2, 47.5, false, null)
        org.junit.Assert.assertNull("a standalone item has no split of its own", legSplit(listOf(home, item(1, 47.0, true, null), roi), 2))
        val after = legSplit(listOf(home, item(1, 47.0, true, null), roi, item(3, 48.0, true, null)), 3) ?: error("split")
        org.junit.Assert.assertEquals("the ROI is skipped when looking back", 47.5, after.latitude, 0.01)
        val survey = item(1, 47.0, true, TrackPoint(47.4, 8.0))
        val fromExit = legSplit(listOf(home, survey, item(2, 48.0, true, null)), 2) ?: error("split")
        org.junit.Assert.assertEquals(47.7, fromExit.latitude, 0.01)
    }
}
