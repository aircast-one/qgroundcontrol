package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class DeviceConnectTest {
    private val served = JSONObject(
        """{"choices":[{"value":"rtsp","label":"QGroundControl","hides":["WHEP"]},{"value":"whep","label":"Browser or Aircast QGC","hides":["RTSP"]},{"value":"all","label":"Show all","hides":[]}],
        "scope":"Every link reaches this device at falcon.tail.ts.net.",
        "cameras":[{"name":"cam2","status":{"level":2,"text":"unreachable"},"notice":"The link below is still the right one.","fix":{"label":"Fix cam2 →","href":"/streaming?camera=cam2"},
          "targets":[{"proto":"RTSP","label":"RTSP · QGroundControl","url":"rtsp://falcon:8554/cam2","hint":"Paste it.","qr":true,"down":true,"alternatives":[{"label":"Ethernet port (eth0)","url":"rtsp://192.168.1.5:8554/cam2"}]},
                     {"proto":"WHEP","label":"WHEP · Browser","url":"http://falcon:8889/cam2/whep","reach":"http://falcon:8889/cam2/whep","qr":true,"alternatives":[]}]}],
        "telemetry":{"status":{"level":0,"text":"connected"},"targets":[{"proto":"UDP","label":"UDP · MAVLink","url":"falcon:14550","qr":false,"alternatives":[]}]},
        "trouble":{"fallback":[{"proto":"HLS","label":"Web players and Safari","url":"http://falcon:8888/cam2/index.m3u8","tag":"High latency","qr":true,"alternatives":[]}]}}""",
    )

    @Test
    fun `reads the links the device serves`() {
        val view = parseConnect(served)
        val camera = view?.cameras?.first()
        assertEquals(DeviceBadge(DeviceLevel.Blocked, "unreachable"), camera?.status)
        assertEquals(PageLink("Fix cam2 →", "/streaming?camera=cam2"), camera?.fix)
        val rtsp = camera?.targets?.first()
        assertEquals(listOf("RTSP", "", "", "true", "true"), listOf(rtsp?.proto, rtsp?.reach, rtsp?.tag, "${rtsp?.down}", "${rtsp?.qr}"))
        assertEquals(listOf(ConnectAddress("Ethernet port (eth0)", "rtsp://192.168.1.5:8554/cam2")), rtsp?.alternatives)
        assertEquals("http://falcon:8889/cam2/whep", camera?.targets?.get(1)?.reach)
        assertEquals("connected", view?.telemetry?.status?.text)
        assertEquals("High latency", view?.trouble?.fallback?.first()?.tag)
        assertNull(view?.trouble?.addresses)
        assertNull(view?.noCamera)
        assertNull(parseConnect(JSONObject("""{"cameras":[]}""")))
        assertEquals(PageLink("Add a camera", "/streaming"), parseConnect(JSONObject("""{"choices":[],"cameras":[],"noCamera":{"label":"Add a camera","href":"/streaming"}}"""))?.noCamera)
    }

    @Test
    fun `the ground station the operator named decides which links are on offer`() {
        val view = parseConnect(served)
        val choices = view?.choices.orEmpty()
        val targets = view?.cameras?.first()?.targets.orEmpty()
        assertEquals("rtsp", chosenGcs(choices, null)?.value)
        assertEquals("rtsp", chosenGcs(choices, "nonsense")?.value)
        assertEquals(listOf("RTSP"), shownTargets(targets, chosenGcs(choices, null)).map { it.proto })
        assertEquals(listOf("WHEP"), shownTargets(targets, chosenGcs(choices, "whep")).map { it.proto })
        assertEquals(listOf("RTSP", "WHEP"), shownTargets(targets, chosenGcs(choices, "all")).map { it.proto })
        assertEquals(listOf("UDP"), shownTargets(view?.telemetry?.targets.orEmpty(), chosenGcs(choices, "whep")).map { it.proto })
        assertNull(chosenGcs(emptyList(), "rtsp"))
    }

    @Test
    fun `a link with no address to probe gets no verdict`() {
        assertTrue(probeBlocked(""))
        assertFalse(probeBlocked("http://falcon:8889/cam2/whep"))
        assertFalse(probeBlocked("https://share.example/w/cam"))
    }
}
