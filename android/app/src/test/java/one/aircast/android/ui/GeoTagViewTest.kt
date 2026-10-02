package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class GeoTagViewTest {

    private fun state(body: String) = geoTagState(JSONObject("""{"kind":"object","class":"GeoTagController",$body}"""))!!

    @Test
    fun `the button reads as GeoTagPage's does`() {
        assertEquals("Cancel", geoTagButton(state(""""inProgress":true,"previewMode":true""")))
        assertEquals("Preview", geoTagButton(state(""""inProgress":false,"previewMode":true""")))
        assertEquals("Start Tagging", geoTagButton(state(""""inProgress":false,"previewMode":false""")))
    }

    @Test
    fun `the summary lists only what went wrong`() {
        assertEquals("Successfully tagged 5 images", geoTagSummary(state(""""taggedCount":5""")))
        assertEquals("Successfully tagged 5 images (2 skipped, 1 failed)", geoTagSummary(state(""""taggedCount":5,"skippedCount":2,"failedCount":1""")))
        assertNull(geoTagSummary(state(""""taggedCount":5,"inProgress":true""")))
        assertNull(geoTagSummary(state(""""taggedCount":0""")))
    }

    @Test
    fun `steps tick once filled and only the core's image types are staged`() {
        assertEquals("✓", geoTagStep(true, 1))
        assertEquals("2", geoTagStep(false, 2))
        assertTrue(isGeoTagImage("IMG_0001.JPG"))
        assertTrue(isGeoTagImage("raw.dng"))
        assertFalse(isGeoTagImage("notes.txt"))
        assertNull(geoTagState(JSONObject("""{"class":"Something"}""")))
    }
}

class GeoTagRunTest {

    @Test
    fun `the count compared is the core's tagged count, which already leaves failures out`() {
        assertNull(publishedNote(4, 4))
        assertEquals("Only 3 of the 4 tagged images could be written to the chosen folder.", publishedNote(3, 4))
    }

    @Test
    fun `the offset reads with either decimal mark and is shown with a point`() {
        assertEquals(2.5, parsedOffset("2,5")!!, 0.0)
        assertEquals(-1.0, parsedOffset(" -1.0 ")!!, 0.0)
        assertNull(parsedOffset("abc"))
        assertEquals("-3.5", shownOffset(-3.5))
    }
}
