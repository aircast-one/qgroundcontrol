package one.aircast.mapspike

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
}
