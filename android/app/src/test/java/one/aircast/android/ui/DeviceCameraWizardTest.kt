package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class DeviceCameraWizardTest {
    private val port = ScanPort("eth0", "Ethernet port (eth0)", "ethernet", "searched", "", 0, "", "", false, emptyList())
    private val swept = port.copy(subnet = "192.168.8.0/24", checked = 253)
    private val uplink = port.copy(iface = "wlan0", label = "Wi-Fi (wlan0)", type = "wi-fi", subnet = "192.168.1.0/24", checked = 253, isUplink = true)
    private val cam = FoundCamera("192.168.8.10", "rtsp://192.168.8.10:554/stream1", needsAuth = false, pathFound = true, used = "")
    private val subnets = listOf(CameraSubnet("192.168.144.2/24", "SIYI, Topotek gimbals, Herelink"), CameraSubnet("192.168.8.2/24", "LTE routers"))
    private val siyi = CameraTemplate("siyi", "SIYI camera", "ZT30 — plugged into this device's Ethernet port", "192.168.144.2/24", "192.168.144.25", "siyi", "siyi", "192.168.144.25:37260")
    private val noFixer = PortFixer(open = false, input = "", suggest = "", status = "", busy = false)

    @Test
    fun `reads the wizard the device serves`() {
        val view = parseCameraWizard(
            JSONObject(
                """{"name":"cam2","new":false,"source":"rtsp://192.168.8.10:554/stream","rtsp":"","rtspHint":"rtsp://192.168.8.10:554/stream","interface":"eth0",
                "interfaces":[{"value":"eth0","label":"Ethernet port (eth0) — 192.168.8.2/24","address":"192.168.8.2/24"}],
                "piCamera":{"note":"A Raspberry Pi camera is connected to this device.","action":"Use it"},
                "others":[{"name":"cam","source":"rtsp://x"}],
                "subnets":[{"cidr":"192.168.144.2/24","label":"SIYI, Topotek gimbals, Herelink"},{"cidr":"192.168.8.2/24","label":"LTE routers"}]}""",
            ),
        )
        assertEquals("eth0", view?.iface)
        assertEquals("192.168.8.2/24", view?.interfaces?.firstOrNull()?.address)
        assertEquals("Use it", view?.piCamera?.action)
        assertEquals(listOf(CameraRef("cam", "rtsp://x")), view?.others)
        assertEquals(subnets, view?.subnets)
        assertEquals(emptyList<CameraTemplate>(), view?.templates)
        assertNull(parseCameraWizard(null))
    }

    @Test
    fun `reads the scan, and a camera this wizard already is reads as fresh`() {
        val scan = JSONObject(
            """{"cameras":[{"host":"192.168.8.4","port":554,"url":"rtsp://192.168.8.4:554/s","pathFound":true,"usedBy":"cam"},
            {"host":"192.168.8.5","port":554,"url":"rtsp://192.168.8.5:554/s","pathFound":true,"usedBy":"cam2"}],
            "ports":[{"interface":"eth1","label":"Ethernet port (eth1)","type":"ethernet","state":"searched","neighbors":[{"addr":"192.168.144.25","mac":"x","suggest":"192.168.144.2/24"}]}]}""",
        )
        assertEquals(listOf("cam", ""), parseFoundCameras(scan, "cam2").map { it.used })
        val first = parseScanPorts(scan).first()
        assertEquals("eth1", first.iface)
        assertEquals(listOf(ScanNeighbor("192.168.144.25", "192.168.144.2/24")), first.neighbors)
    }

    @Test
    fun `takes the host out of a camera URL`() {
        assertEquals("192.168.8.10", rtspHost("rtsp://192.168.8.10:554/stream"))
        assertEquals("cam.local", rtspHost("rtsp://cam.local/live"))
        assertEquals("", rtspHost("not a url"))
        assertEquals("", rtspHost(""))
    }

    @Test
    fun `supplies the scheme, and never rewrites one that is there`() {
        assertEquals("rtsp://192.168.8.10", normalizeRtsp("192.168.8.10"))
        assertEquals("rtsp://cam.local:554/live", normalizeRtsp("cam.local:554/live"))
        assertEquals("rtsp://192.168.8.10/stream", normalizeRtsp("  192.168.8.10/stream  "))
        assertEquals("rtsp://192.168.8.10/x", normalizeRtsp("rtsp://192.168.8.10/x"))
        assertEquals("http://192.168.8.10", normalizeRtsp("http://192.168.8.10"))
        assertEquals("", normalizeRtsp(""))
    }

    @Test
    fun `never offers the address the camera already answers on`() {
        assertEquals("192.168.8.3/24", suggestIP("rtsp://192.168.8.2/live"))
        assertEquals("192.168.8.2/24", suggestIP("rtsp://192.168.8.10/live"))
        assertEquals("", suggestIP("rtsp://cam.local/live"))
        assertEquals("", suggestIP("garbage"))
    }

    @Test
    fun `spots a subnet typed where a camera URL belongs`() {
        assertEquals("192.168.8.0/24", cidrIn("192.168.8.0/24"))
        assertEquals("192.168.8.0/24", cidrIn(" rtsp://192.168.8.0/24 "))
        assertEquals("", cidrIn("192.168.8.10"))
        assertEquals("", cidrIn("rtsp://192.168.8.10/stream"))
        assertEquals("", cidrIn("192.168.8.0/33"))
    }

    @Test
    fun `refuses anything that is not four plain octets`() {
        assertEquals(0L, ipToInt("0.0.0.0"))
        assertEquals(4294967295L, ipToInt("255.255.255.255"))
        assertEquals(3232235777L, ipToInt("192.168.1.1"))
        assertNull(ipToInt("1.2.3"))
        assertNull(ipToInt("1.2.3.256"))
        assertNull(ipToInt("1.2.3."))
        assertNull(ipToInt("1.2.3. 4"))
        assertNull(ipToInt("1.2.3.0x4"))
    }

    @Test
    fun `masks both sides before comparing, and is false on junk`() {
        assertTrue(sameSubnet("192.168.8.10", "192.168.8.0/24"))
        assertFalse(sameSubnet("192.168.9.10", "192.168.8.0/24"))
        assertTrue(sameSubnet("192.168.8.10", "192.168.8.3/24"))
        assertTrue(sameSubnet("10.1.2.3", "10.0.0.0/8"))
        assertTrue(sameSubnet("1.2.3.4", "255.255.255.255/0"))
        assertFalse(sameSubnet("192.168.8.11", "192.168.8.10/32"))
        assertFalse(sameSubnet("cam.local", "192.168.8.0/24"))
        assertFalse(sameSubnet("192.168.8.10", "not-a-subnet"))
        assertFalse(sameSubnet("192.168.8.10", "192.168.8.0/33"))
        assertFalse(sameSubnet("", ""))
    }

    @Test
    fun `names a port by what is wrong with it, not by a missing camera`() {
        assertEquals("checked 253 addresses on 192.168.8.0/24 — nothing answered", portLabel(swept))
        assertEquals("nothing plugged in — check the cable and the camera's power", portLabel(port.copy(state = "no-carrier")))
        assertEquals("not connected to any network", portLabel(port.copy(state = "no-carrier", type = "wi-fi")))
        assertTrue(portLabel(port.copy(state = "failed", reason = "address-unusable", triedAddress = "192.168.8.4/24")).startsWith("192.168.8.4/24 could not be used"))
        assertEquals(
            "a device answered at 192.168.144.25 (not confirmed as a camera), not on 192.168.1.0/24 — set this port to 192.168.144.2/24 to reach it",
            portLabel(port.copy(state = "searched", subnet = "192.168.1.0/24", neighbors = listOf(ScanNeighbor("192.168.144.25", "192.168.144.2/24")))),
        )
        assertEquals("checked 253 addresses on 192.168.1.0/24 — nothing answered — this is the device's own uplink", portDetail(ScannedPort(uplink, false), subnets, ""))
        assertTrue(portDetail(ScannedPort(swept, false), subnets, "192.168.8.10").contains("nothing answered at 192.168.8.10 on eth0"))
        assertEquals("camera found here", portDetail(ScannedPort(swept, true), subnets, ""))
    }

    @Test
    fun `a router on a port already set for the camera asks for the cable, not an address`() {
        val idle = port.copy(subnet = "192.168.144.0/24", neighbors = listOf(ScanNeighbor("192.168.1.3", "")))
        val outcome = scanOutcome(emptyList(), listOf(idle, uplink), "")
        assertEquals("Check the cable", todoHeading(outcome.todo, subnets))
        assertTrue(portDetail(outcome.todo.firstOrNull() ?: ScannedPort(idle, false), subnets, "").contains("Plug the camera in here"))
        assertEquals(false, fixersFor(outcome, subnets)["eth0"]?.open)
        assertTrue(checklistHint(listOf(idle), subnets).contains("looks like a router or switch"))
        assertEquals(listOf("192.168.144.2/24", "192.168.8.2/24"), subnetOptions(idle, listOf(idle, uplink), subnets).map { it.cidr })
    }

    @Test
    fun `scan results lead with the port that has a next step`() {
        val blank = port.copy(state = "no-address")
        val outcome = scanOutcome(emptyList(), listOf(uplink, blank), "")
        assertEquals(listOf("eth0"), outcome.todo.map { it.port.iface })
        assertEquals(listOf("wlan0"), outcome.rest.map { it.port.iface })
        assertEquals("Try this", todoHeading(outcome.todo, subnets))
        assertEquals("No camera answered.", outcome.note)
        assertEquals(PortFixer(open = true, input = "", suggest = "", status = "", busy = false), fixersFor(outcome, subnets)["eth0"])
        assertEquals("No camera port on this device — type the camera's address below.", scanOutcome(emptyList(), emptyList(), "").note)
    }

    @Test
    fun `a port with nothing left to do stops competing once a new camera answers`() {
        val outcome = scanOutcome(listOf(cam), listOf(swept, port.copy(iface = "eth1", state = "no-address")), "")
        assertEquals(emptyList<ScannedPort>(), outcome.todo)
        assertEquals("rtsp://192.168.8.10:554/stream1", adoptedCamera(outcome, true, "")?.url)
    }

    @Test
    fun `only one fresh camera with a path is adopted, and never over a typed address`() {
        assertNull(adoptedCamera(scanOutcome(listOf(cam), listOf(swept), ""), true, "rtsp://192.168.144.25:8554/main.264"))
        assertNull(adoptedCamera(scanOutcome(listOf(cam.copy(pathFound = false)), listOf(swept), ""), false, ""))
        assertNull(adoptedCamera(scanOutcome(listOf(cam.copy(used = "cam")), listOf(swept), ""), false, ""))
        assertNull(adoptedCamera(scanOutcome(listOf(cam, cam.copy(host = "192.168.8.11")), listOf(swept), ""), false, ""))
        assertEquals("192.168.144.25", adoptedCamera(scanOutcome(listOf(cam.copy(used = "cam"), cam.copy(host = "192.168.144.25")), listOf(swept), ""), false, "")?.host)
    }

    @Test
    fun `the subnet chips put what was measured first and never a subnet another port holds`() {
        val failed = port.copy(state = "failed", reason = "address-unusable", triedAddress = "192.168.8.4/24", neighbors = listOf(ScanNeighbor("192.168.144.25", "192.168.144.2/24")))
        assertEquals(
            listOf(CameraSubnet("192.168.144.2/24", "found 192.168.144.25"), CameraSubnet("192.168.8.4/24", "tried before"), CameraSubnet("192.168.8.2/24", "LTE routers")),
            subnetOptions(failed, listOf(failed), subnets),
        )
        val fixer = fixersFor(scanOutcome(emptyList(), listOf(failed), ""), subnets)["eth0"]
        assertEquals(true, fixer?.open)
        assertEquals("192.168.144.2/24", fixer?.input)
        assertEquals("Set up eth0 and find the camera", fixerGoLabel(failed, fixer ?: noFixer))
    }

    @Test
    fun `a camera make picks the port that can carry its subnet and says when another holds it`() {
        assertEquals("eth1", templatePort(listOf(uplink, port.copy(state = "no-carrier"), port.copy(iface = "eth1", state = "no-address")), siyi)?.iface)
        assertNull(templatePort(listOf(uplink), siyi))
        val ready = port.copy(subnet = "192.168.144.0/24")
        assertEquals("eth0", templateReadyPort(siyi, emptyList(), listOf(ready))?.iface)
        assertNull(templateReadyPort(siyi, listOf(cam.copy(host = "192.168.144.25")), listOf(ready)))
        assertEquals("The Ethernet port (eth0) is ready for a SIYI camera — nothing answered at 192.168.144.25. Check the cable.", templateSub(siyi, ready))
        assertEquals(siyi.hint, templateSub(siyi, null))
        assertEquals("Scan again", templateAction(false, ready))
        assertEquals("Setting up…", templateAction(true, ready))
        assertTrue(templateHolderError(uplink, siyi.copy(label = "OpenIPC camera", cidr = "192.168.1.2/24")).contains("Wi-Fi (wlan0) already uses 192.168.1.x"))
    }

    @Test
    fun `a sign-in goes into the URL, encoded, and leaves with the camera`() {
        assertEquals("rtsp://admin:p%40ss%20word@192.168.8.14:554/live", camURL("rtsp://192.168.8.14:554/live", "admin", "p@ss word"))
        assertEquals("rtsp://admin@192.168.8.14:554/live", camURL("rtsp://192.168.8.14:554/live", " admin ", ""))
        assertEquals("rtsp://192.168.8.14:554/live", camURL("rtsp://192.168.8.14:554/live", "", ""))
    }

    @Test
    fun `Scan leads until an address is filled, then Continue takes over`() {
        assertTrue(scanLeads("", false))
        assertFalse(continueLeads(""))
        assertFalse(scanLeads("rtsp://192.168.8.99:554/stream", false))
        assertTrue(continueLeads("rtsp://192.168.8.99:554/stream"))
        assertFalse(scanLeads("", true))
    }

    @Test
    fun `a camera on a working port skips the port question`() {
        assertTrue(commitsNow("rtsp://192.168.8.10:554/stream1", listOf(swept)))
        assertEquals("Set up & verify", nextLabel("rtsp://192.168.8.10:554/stream1", listOf(swept), false))
        assertEquals("Add camera", nextLabel("rtsp://192.168.8.10:554/stream1", listOf(swept), true))
        assertEquals("Continue", nextLabel("rtsp://192.168.50.99:554/s", listOf(swept), true))
        assertEquals("Add camera", nextLabel("rpiCamera", emptyList(), true))
        assertTrue(nextStep(NextInput("rtsp://192.168.8.10:554/stream1", null, emptyList(), listOf(swept), "")) is NextOutcome.Apply)
        assertEquals(NextOutcome.Port("rtsp://192.168.50.99:554/s", null), nextStep(NextInput("192.168.50.99:554/s", null, emptyList(), listOf(swept), "")))
    }

    @Test
    fun `Continue refuses what cannot work and says where it belongs`() {
        val refusal = { rtsp: String, name: String?, dupAck: String ->
            nextStep(NextInput(rtsp, name, listOf(CameraRef("cam", "rtsp://192.168.8.10:554/stream")), emptyList(), dupAck)) as? NextOutcome.Refuse
        }
        assertTrue(refusal("192.168.144.2/24", null, "")?.refusal?.text.orEmpty().contains("address for this device's port, not the camera"))
        assertEquals("rtsp", refusal("rtsp://192.168.144.2/24", null, "")?.refusal?.field)
        assertEquals(WizardRefusal("Give the camera a name.", false, "name"), refusal("rtsp://192.168.8.11/s", " ", "")?.refusal)
        assertEquals("A camera named “cam” already exists — pick another name.", refusal("rtsp://192.168.8.11/s", "CAM", "")?.refusal?.text)
        assertTrue(refusal("ftp://x", null, "")?.refusal?.text.orEmpty().contains("Enter the camera's address"))
        val dup = refusal("rtsp://192.168.8.10:554/stream", null, "")
        assertEquals(true, dup?.refusal?.warn)
        assertEquals("rtsp://192.168.8.10:554/stream", dup?.dupAck)
        assertNull(refusal("rtsp://192.168.8.10:554/stream", null, "rtsp://192.168.8.10:554/stream"))
        assertEquals(NextOutcome.Apply("rpiCamera", "picam"), nextStep(NextInput("rpiCamera", "picam", emptyList(), emptyList(), "")))
    }

    @Test
    fun `the apply note says whether the port changes`() {
        assertEquals("eth0 keeps 192.168.8.2/24 — nothing on that port changes.", applyNote("eth0", "192.168.8.2/24", "192.168.8.2/24"))
        assertEquals("Sets eth0 to 192.168.50.2/24 — anything else on that port may need reconnecting.", applyNote("eth0", "192.168.50.2/24", ""))
        assertEquals("", applyNote("", "192.168.50.2/24", ""))
    }
}
