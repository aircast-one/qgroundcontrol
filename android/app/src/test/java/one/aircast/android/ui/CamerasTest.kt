package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class CamerasTest {
    private val view = JSONObject(
        """
        {"kind":"object","class":"Cameras","readable":true,"reason":null,"active":2,
         "cameras":[
           {"slot":0,"stored":0,"title":"Front","name":"Front","source":"RTSP Video Stream","url":"rtsp://10.0.0.5:8554/front","summary":"RTSP Video Stream · rtsp://10.0.0.5:8554/front","problem":null,"fromDrone":false,"active":false},
           {"slot":1,"stored":1,"title":"Camera 2","name":"","source":"RTSP Video Stream","url":"","summary":"RTSP Video Stream · no address","problem":"This kind of stream needs an address.","fromDrone":false,"active":false},
           {"slot":2,"stored":null,"title":"SIYI A8","name":"SIYI A8","source":"UDP h.264 Video Stream","url":"0.0.0.0:5600","summary":"UDP h.264 Video Stream · 0.0.0.0:5600","problem":null,"fromDrone":true,"active":true}],
         "kinds":[{"raw":"RTSP Video Stream","label":"RTSP Video Stream","group":"Video streams","more":false,"needsUrl":true,"hint":"rtsp://192.168.1.10:8554/live","schemes":["rtsp://","rtsps://"]}]}
        """.trimIndent(),
    )

    @Test
    fun `every camera reads as one list, and only the operator's own can be edited`() {
        val reading = camerasReading(view)!!
        assertEquals(3, reading.cameras.size)
        assertEquals(listOf(0, 1), reading.stored.map { it.stored })
        assertNull(reading.cameras[2].stored)
        assertEquals(true, reading.cameras[2].fromDrone)
        assertEquals("This kind of stream needs an address.", reading.cameras[1].problem)
        assertNull(reading.cameras[0].problem)
        assertEquals("rtsp://192.168.1.10:8554/live", reading.kinds.single().hint)
        assertEquals(listOf("rtsp://", "rtsps://"), reading.kinds.single().schemes)
        assertNull(camerasReading(JSONObject("""{"class":"Video"}""")))
    }

    @Test
    fun `an address that carries its scheme picks the kind of stream it is`() {
        val kinds = listOf(
            CameraKind("RTSP Video Stream", "RTSP Video Stream", "Video streams", true, "", schemes = listOf("rtsp://", "rtsps://")),
            CameraKind("UDP h.264 Video Stream", "UDP h.264 Video Stream", "Video streams", true, "", schemes = listOf("udp://")),
            CameraKind("UDP h.265 Video Stream", "UDP h.265 Video Stream", "Video streams", true, "", schemes = listOf("udp265://", "udp://")),
            CameraKind("WebRTC (WHEP) Video Stream", "WebRTC (WHEP) Video Stream", "Video streams", true, "", schemes = listOf("http://", "https://")),
        )
        assertEquals("WebRTC (WHEP) Video Stream", inferredKind(kinds, "RTSP Video Stream", "https://cam.local/whep"))
        assertEquals("RTSP Video Stream", inferredKind(kinds, "UDP h.264 Video Stream", "RTSP://10.0.0.5/live"))
        assertEquals("a kind that accepts the scheme is kept", "UDP h.265 Video Stream", inferredKind(kinds, "UDP h.265 Video Stream", "udp://0.0.0.0:5600"))
        assertEquals("no scheme, no guess", "RTSP Video Stream", inferredKind(kinds, "RTSP Video Stream", "10.0.0.5:8554/live"))
        assertEquals("an unknown scheme changes nothing", "RTSP Video Stream", inferredKind(kinds, "RTSP Video Stream", "ftp://x"))
    }

    @Test
    fun `the settings index names the camera on screen and how many there are`() {
        assertEquals("SIYI A8 · 3 cameras", camerasGlance(camerasReading(view)))
        assertEquals("", camerasGlance(null))
        assertEquals("RTSP", kindLabel("RTSP Video Stream"))
    }
}
