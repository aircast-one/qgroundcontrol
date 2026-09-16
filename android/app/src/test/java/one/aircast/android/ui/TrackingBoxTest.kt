package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class TrackingBoxTest {

    private fun camera(
        supported: Boolean = true,
        active: Boolean = true,
        rect: String = """{"x":0.35,"y":0.30,"width":0.30,"height":0.40}""",
        shapes: String = """["rectangle","point"]""",
    ) = JSONObject(
        """{"kind":"object","class":"Camera","present":true,
            "tracking":{"supported":$supported,"enabled":true,"active":$active,
                        "shapes":$shapes,"rect":$rect}}""",
    )

    @Test
    fun `a camera that cannot track offers nothing`() {
        assertNull(trackingReading(camera(supported = false)))
        assertNull(trackingReading(JSONObject("""{"kind":"object","class":"Camera"}""")))
        assertNull(trackingReading(null))
    }

    @Test
    fun `the box is the served rectangle`() {
        val box = trackingReading(camera())!!.box!!

        assertEquals(0.35, box.x, 1e-9)
        assertEquals(0.30, box.y, 1e-9)
        assertEquals(0.30, box.width, 1e-9)
        assertEquals(0.40, box.height, 1e-9)
    }

    @Test
    fun `a rectangle with no area is not a box`() {
        assertNull(
            "the bridge answers a QRectF for an invalid rect as null, but a zero-sized one " +
                "would draw as a point-sized frame the operator cannot see and cannot dismiss",
            trackingReading(camera(rect = """{"x":0.5,"y":0.5,"width":0.0,"height":0.0}"""))!!.box,
        )
        assertNull(trackingReading(camera(rect = "null"))!!.box)
    }

    @Test
    fun `stopping is offered only while something is being tracked`() {
        assertTrue(trackingCanStop(trackingReading(camera(active = true))))
        assertFalse(trackingCanStop(trackingReading(camera(active = false, rect = "null"))))
        assertFalse(trackingCanStop(null))
    }

    @Test
    fun `starting needs a shape the camera accepts`() {
        assertTrue(trackingCanStart(trackingReading(camera(active = false, rect = "null"))))
        assertFalse(
            "already tracking, so the next command is stop",
            trackingCanStart(trackingReading(camera(active = true))),
        )
        assertFalse(
            "a camera claiming tracking but naming no shape cannot be told how to start",
            trackingCanStart(trackingReading(camera(active = false, rect = "null", shapes = "[]"))),
        )
    }

    @Test
    fun `the rectangle payload is the shape the bridge now accepts`() {
        val sent = trackingRectObject(TRACKING_CENTRE)

        assertEquals(
            "QGCBridgeCore builds a QRectF from exactly these four keys and refuses when one " +
                "is missing - the key set is the contract, and JSONObject does not keep the " +
                "order they were put in, so a test on its toString pins nothing useful",
            setOf("x", "y", "width", "height"),
            sent.keys().asSequence().toSet(),
        )
        assertEquals(0.4, sent.getDouble("x"), 1e-9)
        assertEquals(0.4, sent.getDouble("y"), 1e-9)
        assertEquals(0.2, sent.getDouble("width"), 1e-9)
        assertEquals(0.2, sent.getDouble("height"), 1e-9)
    }
}
