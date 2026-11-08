package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class AircastLinkIndicatorTest {
    @Test
    fun `the link shows only once cellular telemetry arrives`() {
        assertNull(aircastLink(JSONObject("""{"shown":false}""")))
        val link = aircastLink(JSONObject("""{"shown":true,"qualityText":"73%","signalText":"73 %","network":"LTE","modem":"connected","bitrateText":"2.5 Mbit/s","qualityHistory":[73,-1],"bitrateHistory":[2048,0]}""")) ?: error("shown")
        assertEquals(listOf("73%", "LTE", "connected", "2.5 Mbit/s"), listOf(link.cellText, link.network, link.modem, link.bitrateText))
        assertEquals(listOf(73.0, -1.0), link.qualityHistory)
        assertEquals("", link.uplinkText)
        assertEquals(emptyList<LinkSwitch>(), link.switches)
        assertEquals(listOf("Signal:", "Network:", "Modem:", "Video bitrate:"), aircastLinkRows(link).map { it.first })
    }

    @Test
    fun `the cell names the uplink in use and the sheet lists its switches newest first`() {
        val link = aircastLink(JSONObject("""{"shown":true,"quality":null,"qualityText":"--","signalText":"unknown","bitrateText":"1.2 Mbit/s","cellular":true,"uplink":"wifi","uplinkText":"Wi-Fi","cellText":"Wi-Fi","switches":[{"atMs":2000,"clock":"21:04:09","from":"LTE","to":"Wi-Fi"},{"atMs":1000,"clock":"21:03:41","from":"Starlink","to":"LTE"}]}""")) ?: error("shown")
        assertEquals(listOf("Wi-Fi", "Wi-Fi"), listOf(link.cellText, link.uplinkText))
        assertEquals(listOf(LinkSwitch("21:04:09", "LTE", "Wi-Fi"), LinkSwitch("21:03:41", "Starlink", "LTE")), link.switches)
        assertEquals("Uplink:" to "Wi-Fi", aircastLinkRows(link).first())
        assertEquals("", aircastLink(JSONObject("""{"shown":true,"uplink":null,"uplinkText":"","cellText":"87%"}"""))?.uplinkText)
    }

    @Test
    fun `an aircraft with no modem shows only its uplink and no bitrate`() {
        val link = aircastLink(JSONObject("""{"shown":true,"quality":null,"qualityText":"--","bitrateText":"","cellular":false,"uplink":"starlink","uplinkText":"Starlink","cellText":"Starlink","switches":[]}""")) ?: error("shown")
        assertEquals(listOf("Uplink:" to "Starlink"), aircastLinkRows(link))
        assertEquals("", link.bitrateText)
    }

    @Test
    fun `a sparkline breaks at unknown samples and scales to its peak`() {
        val runs = sparkRuns(listOf(50.0, 100.0, -1.0, 0.0, 25.0, -1.0, 10.0), maximum = 0.0)
        assertEquals(2, runs.size)
        assertEquals(listOf(0f to 0.5f, (1f / 6) to 0f), runs[0])
        assertEquals(listOf(0.5f to 1f, (4f / 6) to 0.75f), runs[1])
        assertEquals(emptyList<List<Pair<Float, Float>>>(), sparkRuns(listOf(5.0), 100.0))
    }
}
