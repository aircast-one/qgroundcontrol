package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class DeviceTelemetryTest {
    @Test
    fun `reads the telemetry view the device serves`() {
        val view = parseTelemetry(
            JSONObject(
                """{"status":{"level":0,"text":"ready"},"notices":[{"level":3,"text":"No flight controller is attached."}],
                "endpoints":[{"proto":"UDP","label":"MAVLink","url":"falcon:14550","down":true}],
                "cloud":{"status":{"level":0,"text":"Carrying"},"text":"Also carrying telemetry."},
                "settings":{"endpoints":"udps:0.0.0.0:14550","baud":"57600"},
                "schemes":[{"value":"udps","label":"udps","host":"0.0.0.0","port":"14550","hostLabel":"Address","placeholder":"","hint":"udps hint"}],
                "gimbal":{"title":"SIYI gimbal camera","status":{"level":0,"text":"connected"},"vendor":"siyi","vendors":[{"value":"siyi","label":"SIYI"}],"models":[]}}""",
            ),
        )
        assertEquals(DeviceBadge(DeviceLevel.Ready, "ready"), view?.status)
        assertEquals("falcon:14550", view?.endpoints?.first()?.url)
        assertTrue(view?.endpoints?.first()?.down == true)
        assertNull(view?.endpointNotice)
        assertEquals("Carrying", view?.cloud?.status?.text)
        assertEquals(TelemetrySettings("udps:0.0.0.0:14550", "57600"), view?.settings)
        assertEquals("udps", view?.schemes?.optJSONObject(0)?.optString("value"))
        assertEquals(listOf(DeviceOption("siyi", "SIYI")), view?.gimbal?.vendors)
        assertNull(parseTelemetry(JSONObject("""{"endpoints":[]}""")))
        val endpoint = DeviceEndpoint("UDP", "MAVLink", "falcon:14550", "", "", "", down = false, primary = false)
        assertEquals("UDP · MAVLink", endpointLabel(endpoint))
        assertEquals("TCP · Betaflight", endpointLabel(endpoint.copy(proto = "TCP", label = "TCP · Betaflight")))
    }

    @Test
    fun `reads the endpoint rows the core makes of the stored spec or the typed rows`() {
        val editor = parseEndpointEditor(
            JSONObject(
                """{"ok":true,"result":{
                "rows":[{"scheme":"udpc","host":"","port":"14550","hostLabel":"Ground station address","placeholder":"192.168.1.20","note":"Pushes telemetry.",
                         "problem":"Needs an address. It is not saved until it has one.",
                         "options":[{"value":"udps","label":"UDP · listen","row":{"scheme":"udps","host":"0.0.0.0","port":"14550"}}]}],
                "blank":{"scheme":"udps","host":"0.0.0.0","port":"14550","hostLabel":"Listen address","placeholder":"0.0.0.0","note":"Listens.","problem":"","options":[]},
                "endpoints":"udps:0.0.0.0:14550"}}""",
            ),
        )
        assertEquals(
            EndpointRowView(
                EndpointRow("udpc", "", "14550"),
                "Ground station address",
                "192.168.1.20",
                "Pushes telemetry.",
                "Needs an address. It is not saved until it has one.",
                listOf(EndpointOption("udps", "UDP · listen", EndpointRow("udps", "0.0.0.0", "14550"))),
            ),
            editor?.rows?.single(),
        )
        assertEquals(EndpointRow("udps", "0.0.0.0", "14550"), editor?.blank?.row)
        assertEquals("udps:0.0.0.0:14550", editor?.endpoints)
        assertNull(parseEndpointEditor(JSONObject("""{"ok":false,"reason":"no"}""")))
        assertNull(parseEndpointEditor(null))
    }
}
