package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class DeviceTelemetryTest {
    private fun scheme(value: String, host: String, port: String) = MavlinkScheme(value, value, host, port, "Address", "", "$value hint")

    private val schemes = listOf(scheme("udps", "0.0.0.0", "14550"), scheme("udpc", "", "14550"), scheme("udpb", "255.255.255.255", "14550"), scheme("tcps", "0.0.0.0", "5760"), scheme("tcpc", "", "5760"))

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
        assertEquals("57600", view?.settings?.baud)
        assertEquals(listOf(schemes.first()), view?.schemes)
        assertEquals(listOf(DeviceOption("siyi", "SIYI")), view?.gimbal?.vendors)
        assertNull(parseTelemetry(JSONObject("""{"endpoints":[]}""")))
        val endpoint = DeviceEndpoint("UDP", "MAVLink", "falcon:14550", "", "", "", down = false, primary = false)
        assertEquals("UDP · MAVLink", endpointLabel(endpoint))
        assertEquals("TCP · Betaflight", endpointLabel(endpoint.copy(proto = "TCP", label = "TCP · Betaflight")))
    }

    @Test
    fun `stored endpoints come back as rows a form can edit`() {
        assertEquals(listOf(EndpointRow("udps", "0.0.0.0", "14550"), EndpointRow("tcpc", "gcs.local", "5760")), parseEndpointSpecs("udps:0.0.0.0:14550\n\n tcpc:gcs.local:5760 "))
    }

    @Test
    fun `an IPv6 address keeps its brackets and loses only the port`() {
        assertEquals(listOf(EndpointRow("udps", "[::]", "14550")), parseEndpointSpecs("udps:[::]:14550"))
    }

    @Test
    fun `rows serialize back to the specs the router parses`() {
        assertEquals("udps:0.0.0.0:14550\nudpc:10.0.0.5:14551", serializeEndpoints(schemes, listOf(EndpointRow("udps", "0.0.0.0", "14550"), EndpointRow("udpc", " 10.0.0.5 ", " 14551 "))))
    }

    @Test
    fun `a row with no port is dropped, and an empty host falls back to the default`() {
        assertEquals("udps:0.0.0.0:14550", serializeEndpoints(schemes, listOf(EndpointRow("udpc", "10.0.0.5", ""), EndpointRow("udps", "", "14550"))))
    }

    @Test
    fun `a new row is ready to save as it stands`() {
        assertEquals("udps:0.0.0.0:14550", serializeEndpoints(schemes, listOf(newRow(schemes))))
        assertEquals("tcps:0.0.0.0:5760", serializeEndpoints(schemes, listOf(newRow(schemes, "tcps"))))
    }

    @Test
    fun `retargeting keeps a typed address and replaces an untouched default`() {
        assertEquals(EndpointRow("tcpc", "10.0.0.5", "14550"), retargetRow(schemes, EndpointRow("tcpc", "10.0.0.5", "14550")))
        assertEquals(EndpointRow("udpb", "255.255.255.255", "14550"), retargetRow(schemes, EndpointRow("udpb", "0.0.0.0", "14550")))
        assertEquals(EndpointRow("tcps", "0.0.0.0", "5760"), retargetRow(schemes, EndpointRow("tcps", "", "")))
    }

    @Test
    fun `a half-written address is named in the row, not sent as a broken spec`() {
        assertEquals("Needs an address.", endpointError(schemes, EndpointRow("udpc", "", "14550")))
        assertEquals("Needs a port between 1 and 65535.", endpointError(schemes, EndpointRow("udpc", "10.0.0.5", "")))
        assertEquals("", endpointError(schemes, EndpointRow("udps", "", "14550")))
        assertEquals("", endpointError(schemes, EndpointRow("udpc", "", "")))
        assertEquals("udps:0.0.0.0:14550", serializeEndpoints(schemes, listOf(EndpointRow("udpc", "", "14550"), EndpointRow("udps", "", "14550"))))
    }

    @Test
    fun `a port that is not a port is named in the row`() {
        assertEquals("Needs a port between 1 and 65535.", endpointError(schemes, EndpointRow("udps", "0.0.0.0", "ssh")))
        assertEquals("Needs a port between 1 and 65535.", endpointError(schemes, EndpointRow("udps", "0.0.0.0", "70000")))
        assertEquals("", endpointError(schemes, EndpointRow("udps", "0.0.0.0", "14550")))
        assertEquals("", serializeEndpoints(schemes, listOf(EndpointRow("udps", "", "ssh"))))
    }

    @Test
    fun `a scheme the device does not offer is shown as itself and saved untouched`() {
        val row = EndpointRow("serial", "/dev/ttyUSB0", "57600")
        assertEquals(DeviceOption("serial", "serial"), schemeOptions(schemes, row).last())
        assertEquals(5, schemeOptions(schemes, EndpointRow("udps", "", "")).size)
        assertEquals("serial:/dev/ttyUSB0:57600", serializeEndpoints(schemes, listOf(row)))
        assertEquals(UNKNOWN_SCHEME_HINT, describeEndpoint(schemes, row))
        assertEquals("Needs an address.", endpointError(schemes, EndpointRow("serial", "", "57600")))
    }
}
