package one.aircast.android.ui

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class DeviceSetupTest {
    private val served = JSONObject(
        """
        {
          "start": 3,
          "steps": [
            {"key": "network", "label": "Internet", "title": "Connect to the internet", "lead": "This device needs internet.", "status": {"level": 0, "text": "ready"}},
            {"key": "tailscale", "label": "Remote access", "title": "Remote access", "lead": "Reach it from anywhere."},
            {"key": "camera", "label": "Camera", "title": "Camera", "lead": "Point it at the camera."},
            {"key": "telemetry", "label": "Telemetry", "title": "Telemetry", "lead": "Plug the flight controller in."},
            {"key": "done", "label": "Done", "title": "Almost there", "lead": "One item still needs attention."}
          ],
          "internet": {
            "online": {"level": 0, "text": "Online — this device can reach the internet."},
            "joinOpen": false,
            "reopen": {"host": "falcon.local", "hostname": "falcon"},
            "connection": {"title": "Connection", "href": "/network", "icon": "network", "summary": "home-wifi"},
            "hint": "You can continue either way."
          },
          "remote": {"status": {"level": 2, "text": "offline"}, "summary": "no remote access", "facts": [], "addresses": [], "cancel": false, "connect": "Connect to Tailscale"},
          "camera": {
            "problem": "Camera offline — the source is saved but no video is arriving.",
            "wizard": {"name": "cam", "new": false, "source": "rtsp://cam", "interfaces": [], "transports": [], "others": [], "templates": [], "subnets": []},
            "cameras": {"title": "Cameras", "href": "/streaming", "icon": "video", "summary": "1 camera, 1 dark", "needs": true},
            "hint": "The rest lives on Cameras."
          },
          "telemetry": {
            "waiting": {"level": 3, "text": "Waiting for a flight controller on USB."},
            "endpoints": [{"proto": "udps", "label": "MAVLink", "url": "falcon.tail.ts.net:14550"}],
            "hint": "Point your ground station at this UDP address."
          },
          "checks": [
            {"label": "Internet", "status": {"level": 0, "text": "ready"}, "step": 1},
            {"label": "Camera stream", "status": {"level": 3, "text": "not set up"}, "step": 3}
          ]
        }
        """,
    )

    private fun servedWith(key: String, value: Any): JSONObject = JSONObject(served.toString()).put(key, value)

    @Test
    fun `reads the walkthrough the device serves`() {
        val view = parseDeviceSetup(served)
        assertEquals(3, view?.start)
        assertEquals(listOf("network", "tailscale", "camera", "telemetry", "done"), view?.steps?.map { it.key })
        assertEquals(DeviceBadge(DeviceLevel.Ready, "ready"), view?.steps?.get(0)?.status)
        assertNull(view?.steps?.get(1)?.status)
        assertEquals(WifiReopen("falcon.local", "falcon"), view?.internet?.reopen)
        assertEquals("/network", view?.internet?.connection?.href)
        assertEquals("Connect to Tailscale", view?.remote?.connect)
        assertEquals(false, view?.camera?.wizard?.isNew)
        assertEquals("", view?.camera?.watch)
        assertEquals(true, view?.camera?.cameras?.needs)
        assertTrue(view?.telemetry?.waiting?.text.orEmpty().contains("Waiting for a flight controller"))
        assertEquals("falcon.tail.ts.net:14550", view?.telemetry?.endpoints?.firstOrNull()?.url)
        assertEquals(SetupCheckRow("Camera stream", DeviceBadge(DeviceLevel.Neutral, "not set up"), 3), view?.checks?.get(1))
    }

    @Test
    fun `refuses a body that is not a walkthrough, and keeps a stray start on the trail`() {
        assertNull(parseDeviceSetup(null))
        assertNull(parseDeviceSetup(servedWith("steps", JSONArray())))
        assertNull(parseDeviceSetup(servedWith("camera", JSONObject())))
        assertEquals(5, parseDeviceSetup(servedWith("start", 9))?.start)
        assertEquals(1, parseDeviceSetup(servedWith("start", 0))?.start)
        assertEquals(5, clampStep(6, 5))
    }

    @Test
    fun `the footer offers what each step allows`() {
        assertEquals(SetupNav(back = false, later = true, next = SetupNext.Next, finish = false), setupNav(1, 5, false))
        assertEquals(SetupNav(back = true, later = true, next = SetupNext.Next, finish = false), setupNav(3, 5, false))
        assertEquals(SetupNav(back = true, later = false, next = null, finish = true), setupNav(5, 5, false))
    }

    @Test
    fun `a camera being edited holds the wizard until it is applied or skipped`() {
        assertTrue(cameraHoldsTheWizard("camera", true, 1))
        assertTrue(cameraHoldsTheWizard("camera", true, 2))
        assertFalse(cameraHoldsTheWizard("camera", true, 3))
        assertFalse(cameraHoldsTheWizard("camera", false, 1))
        assertFalse(cameraHoldsTheWizard("telemetry", true, 1))
        assertEquals(SetupNav(back = false, later = true, next = SetupNext.Skip, finish = false), setupNav(3, 5, true))
    }
}
