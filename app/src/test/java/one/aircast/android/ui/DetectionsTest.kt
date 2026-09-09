package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class DetectionsTest {

    private val served = """
        {"kind":"object","class":"Detections","available":true,"host":"10.0.0.4","camera":"front",
         "stale":false,"ageMs":40,
         "boxes":[{"x":0.1,"y":0.2,"w":0.3,"h":0.4,"label":"car","confidence":0.91,"target":true},
                  {"x":0.5,"y":0.5,"w":0.1,"h":0.1,"label":"person","confidence":0.42,"target":false}],
         "track":null,"error":null}
    """

    @Test
    fun `a frame carries its boxes with the target marked`() {
        val reading = detections(JSONObject(served))!!

        assertEquals(listOf("car", "person"), reading.boxes.map { it.label })
        assertEquals(listOf(true, false), reading.boxes.map { it.target })
        assertEquals(0.1, reading.boxes[0].x, 1e-9)
        assertEquals(0.4, reading.boxes[0].h, 1e-9)
        assertNull(reading.error)
    }

    @Test
    fun `a reply that is not the detections view reads as nothing`() {
        assertNull(detections(null))
        assertNull(detections(JSONObject("""{"kind":"null"}""")))
    }

    @Test
    fun `nothing is drawn when the feed is stale or unconfigured`() {
        val stale = JSONObject(served).put("stale", true)
        val unconfigured = JSONObject(served).put("available", false)

        assertTrue(visibleBoxes(detections(stale)).isEmpty())
        assertTrue(visibleBoxes(detections(unconfigured)).isEmpty())
        assertEquals(2, visibleBoxes(detections(JSONObject(served))).size)
    }

    @Test
    fun `a configured feed reports its trouble and an unconfigured one stays quiet`() {
        val broken = JSONObject(served).put("error", "connection refused")
        val unconfigured = JSONObject(served).put("available", false).put("error", "connection refused")

        assertEquals("connection refused", detectionTrouble(detections(broken)))
        assertNull(detectionTrouble(detections(unconfigured)))
        assertNull(detectionTrouble(detections(JSONObject(served))))
    }

    @Test
    fun `a caption names what was seen and how sure the detector is`() {
        val reading = detections(JSONObject(served))!!

        assertEquals("car 91%", boxCaption(reading.boxes[0]))
        assertEquals("person 42%", boxCaption(reading.boxes[1]))
        assertEquals("", boxCaption(reading.boxes[0].copy(label = "", confidence = 0.0)))
        assertEquals("car", boxCaption(reading.boxes[0].copy(confidence = 0.0)))
    }
}
