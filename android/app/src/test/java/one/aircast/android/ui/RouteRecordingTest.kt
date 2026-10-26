package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class RouteRecordingTest {
    private fun view(fields: String) = JSONObject("""{"kind":"object","class":"RouteRecorder",$fields}""")

    @Test
    fun `a recording shows the core's pill text and no saved notice`() {
        val recording = routeRecorder(view(""""available":true,"recording":true,"text":"Recording route · 123 m","savedFile":null,"savedName":null,"message":null"""))!!
        assertTrue(recording.recording)
        assertEquals("Recording route · 123 m", recording.text)
        assertNull(recording.message)
        assertNull(routeRecorder(JSONObject("""{"class":"Orbit"}""")))
    }

    @Test
    fun `a stopped recording keeps the saved route until it is dismissed`() {
        val saved = routeRecorder(view(""""available":true,"recording":false,"text":null,"savedFile":"/m/Route 2026-10-24 10-00-00.kml","savedName":"Route 2026-10-24 10-00-00","message":"Route saved""""))!!
        assertNull(saved.text)
        assertEquals("/m/Route 2026-10-24 10-00-00.kml", saved.savedFile)
        assertEquals("Route 2026-10-24 10-00-00", saved.savedName)
        assertEquals("Route saved", saved.message)
        val tooShort = routeRecorder(view(""""available":true,"recording":false,"savedFile":null,"message":"Route not saved: the vehicle didn't fly far enough while recording.""""))!!
        assertNull(tooShort.savedFile)
    }

    @Test
    fun `a refused rename keeps the old file and says the name is taken`() {
        val taken = renamed(JSONObject("""{"ok":false,"refusal":"nameUsed","reason":"Name already used","path":"/m/a.kml"}"""), "/m/a.kml")
        assertEquals(Renamed("/m/a.kml", "Name already used"), taken)
        assertEquals(Renamed("/m/Tower 7.kml", null), renamed(JSONObject("""{"ok":true,"path":"/m/Tower 7.kml"}"""), "/m/a.kml"))
    }

    @Test
    fun `the fly controls offer record or stop, and nothing without a vehicle`() {
        val idle = routeTile(RouteRecorder(available = true, recording = false, text = null, savedFile = null, savedName = null, message = null)) {}!!
        assertEquals("Record route", idle.label)
        assertFalse(idle.warning)
        val running = routeTile(RouteRecorder(available = true, recording = true, text = "Recording route · 5 m", savedFile = null, savedName = null, message = null)) {}!!
        assertEquals("Stop recording", running.label)
        assertNull(routeTile(RouteRecorder(available = false, recording = false, text = null, savedFile = null, savedName = null, message = null)) {})
        assertNull(routeTile(null) {})
    }
}
