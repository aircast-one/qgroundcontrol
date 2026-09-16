package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class TrackingDragTest {

    private val letterboxed = PaintedRect(left = 100.0, top = 0.0, width = 400.0, height = 300.0)

    @Test
    fun `a drag is measured against the picture, not the surface`() {
        val request = trackingRequest(200.0, 60.0, 300.0, 150.0, letterboxed)

        assertTrue(request is TrackingRequest.Box)
        val box = (request as TrackingRequest.Box).rect
        assertEquals(
            "FlyViewVideo subtracts the black stripes before normalising, so an x of 200 on a " +
                "picture starting at 100 is a quarter across the image and not two fifths of " +
                "the surface",
            0.25,
            box.x,
            1e-9,
        )
        assertEquals(0.2, box.y, 1e-9)
        assertEquals(0.25, box.width, 1e-9)
        assertEquals(0.3, box.height, 1e-9)
    }

    @Test
    fun `a drag backwards is the same rectangle as a drag forwards`() {
        val forward = trackingRequest(200.0, 60.0, 300.0, 150.0, letterboxed)
        val backward = trackingRequest(300.0, 150.0, 200.0, 60.0, letterboxed)

        assertEquals(forward, backward)
    }

    @Test
    fun `a tap is a point with a radius, not a rectangle with no area`() {
        val request = trackingRequest(300.0, 150.0, 303.0, 152.0, letterboxed)

        assertTrue(
            "Qt sends the point message when the drag is under ten pixels in both axes; a " +
                "rectangle of nearly no area would ask the camera to track nothing",
            request is TrackingRequest.Point,
        )
        val point = request as TrackingRequest.Point
        assertEquals(0.5, point.x, 1e-9)
        assertEquals(0.5, point.y, 1e-9)
        assertEquals("the radius is a fraction of the picture's width", 0.125, point.radius, 1e-9)
    }

    @Test
    fun `a drag beyond the picture is clamped to it`() {
        val box = (trackingRequest(-500.0, -500.0, 5000.0, 5000.0, letterboxed) as TrackingRequest.Box).rect

        assertEquals(0.0, box.x, 1e-9)
        assertEquals(0.0, box.y, 1e-9)
        assertEquals(1.0, box.width, 1e-9)
        assertEquals(1.0, box.height, 1e-9)
    }

    @Test
    fun `no picture is nothing to aim at`() {
        assertNull(trackingRequest(1.0, 1.0, 2.0, 2.0, PaintedRect(0.0, 0.0, 0.0, 0.0)))
    }

    @Test
    fun `the point payload carries the radius beside the point, so arity picks the overload`() {
        val sent = trackingPointObject(TrackingRequest.Point(0.5, 0.25, 0.1))

        assertEquals(
            "QPointF is built from x and y alone - the radius rides beside it as the second " +
                "argument, and it is the count of arguments that selects this overload over " +
                "the rectangle one",
            setOf("x", "y"),
            sent.keys().asSequence().toSet(),
        )
        assertEquals(0.5, sent.getDouble("x"), 1e-9)
        assertEquals(0.25, sent.getDouble("y"), 1e-9)
    }
}
