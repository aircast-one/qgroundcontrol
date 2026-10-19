package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class DeviceCameraWizardTest {
    @Test
    fun `reads the wizard the device serves`() {
        val view = parseCameraWizard(
            JSONObject(
                """{"name":"cam2","new":false,"source":"rtsp://192.168.8.10:554/stream","rtsp":"","rtspHint":"rtsp://192.168.8.10:554/stream","interface":"eth0",
                "interfaces":[{"value":"eth0","label":"Ethernet port (eth0) — 192.168.8.2/24","address":"192.168.8.2/24"}],
                "piCamera":{"note":"A Raspberry Pi camera is connected to this device.","action":"Use it"},
                "others":[{"name":"cam","source":"rtsp://x"}]}""",
            ),
        )
        assertEquals("eth0", view?.iface)
        assertEquals("192.168.8.2/24", view?.interfaces?.firstOrNull()?.address)
        assertEquals("Use it", view?.piCamera?.action)
        assertEquals("cam", view?.json?.optJSONArray("others")?.optJSONObject(0)?.optString("name"))
        assertNull(parseCameraWizard(null))
    }

    @Test
    fun `reads what the core made of the scan, the ports and the running setup`() {
        val wizard = parseDeviceWizard(
            JSONObject(
                """{"session":3,"scanning":false,"scans":2,"note":"","checklist":"No camera answered — check it, then scan again",
                "adopt":{"host":"192.168.8.10","url":"rtsp://192.168.8.10:554/stream1","needsAuth":false,"pathFound":true,"used":"","note":"ready to use","action":"Use"},
                "cams":[],"camHeading":"0 cameras","camsHint":"","details":"Details for a technician","todoHeading":"Try this",
                "todo":[{"iface":"eth0","label":"Ethernet port (eth0)","detail":"no address on this port","open":true,"suggest":"192.168.144.2/24","hint":"Prefilled.","go":"Set up eth0 and find the camera",
                  "options":[{"cidr":"192.168.144.2/24","text":"192.168.144.2/24 · found 192.168.144.25"}],"busy":true,"status":"Setting up eth0…"}],
                "rest":[{"iface":"wlan0","label":"Wi-Fi (wlan0)","detail":"this is the device's own uplink"}],"restTitle":"Wi-Fi (wlan0) — nothing answered",
                "templates":[{"id":"siyi","label":"SIYI camera","sub":"ZT30","action":"Set up"}],"templateBusy":false,"templateNote":"",
                "template":{"n":1,"refusal":"","camera":null,"name":"siyi"},
                "apply":{"n":0,"state":"failed","text":"That name is taken","field":"name","url":"","camera":""},
                "notice":{"n":1,"text":"Gimbal control was not switched to this camera — no answer"}}""",
            ),
        )
        assertEquals(3L, wizard.session)
        assertEquals(2, wizard.scans)
        assertEquals("rtsp://192.168.8.10:554/stream1", wizard.adopt?.url)
        assertEquals(WizardChip("192.168.144.2/24", "192.168.144.2/24 · found 192.168.144.25"), wizard.todo.first().options.first())
        assertTrue(wizard.todo.first().busy)
        assertEquals("this is the device's own uplink", wizard.rest.first().detail)
        assertEquals(WizardMake("siyi", "SIYI camera", "ZT30", "Set up"), wizard.templates.first())
        assertEquals(WizardFound(1, "", null, "siyi"), wizard.template)
        assertEquals(WizardApply(0, "failed", "That name is taken", "name", "", ""), wizard.apply)
        assertEquals(1, wizard.notice.n)
        assertEquals(-1L, parseDeviceWizard(null).session)
    }

    @Test
    fun `reads what Continue does next`() {
        val refused = parseWizardNext(
            JSONObject("""{"kind":"refuse","rtsp":"rtsp://192.168.8.10:554/stream","name":null,"camera":"cam","refusal":{"text":"Already used","warn":true,"field":null},"dupAck":"rtsp://192.168.8.10:554/stream","iface":"","ip":""}"""),
        )
        assertEquals(WizardRefusal("Already used", true, null), refused?.refusal)
        assertNull(refused?.name)
        val port = parseWizardNext(JSONObject("""{"kind":"port","rtsp":"rtsp://192.168.50.99/s","name":"cam","camera":"cam","refusal":null,"dupAck":"","iface":"eth0","ip":"192.168.50.2/24"}"""))
        assertEquals(WizardNext("port", "rtsp://192.168.50.99/s", "cam", "cam", null, "", "eth0", "192.168.50.2/24"), port)
        assertEquals(WizardForm("Add camera", true, ""), parseWizardForm(JSONObject("""{"next":"Add camera","skipsPort":true,"applyNote":""}""")))
        assertEquals(NO_FORM, parseWizardForm(null))
    }

    @Test
    fun `Scan leads until an address is filled, then Continue takes over`() {
        assertTrue(scanLeads("", false))
        assertFalse(continueLeads(""))
        assertFalse(scanLeads("rtsp://192.168.8.99:554/stream", false))
        assertTrue(continueLeads("rtsp://192.168.8.99:554/stream"))
        assertFalse(scanLeads("", true))
    }
}
