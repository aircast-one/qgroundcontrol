package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Test

class DeviceSettingsTest {
    private val served = JSONObject(
        """
        {
          "health": {"level": 2, "text": "Mobile data down", "href": "/network#cellular"},
          "notice": {"level": 1, "text": "v0.9.1 is available — install it under Software."},
          "stats": [{"label": "Services", "value": "All running", "hint": "3 supervised units", "level": 0}],
          "services": [{"label": "Video server", "state": "running", "level": 0}],
          "cloud": {"status": {"level": 3, "text": "waiting for sign-in"}, "stage": "signing-in", "userCode": "ABCD-1234", "signInUrl": "https://app.aircast.one/device", "issuer": "https://api.aircast.one"},
          "software": {
            "status": {"level": 1, "text": "Update failed"},
            "summary": "v0.9.0",
            "notices": [{"level": 2, "title": "The update to v0.9.1 did not finish.", "text": "You can try again.", "detailLabel": "What went wrong", "detail": "E: dpkg"}],
            "candidate": {"title": "v0.9.1 is available", "notesUrl": "https://aircast.one/docs/releases#v0.9.1", "install": "Install v0.9.1", "confirm": "Install v0.9.1?", "success": "Installing v0.9.1"},
            "notes": ["Updates come from the Stable channel."],
            "checked": "Checked just now.",
            "canCheck": true
          },
          "hostname": "falcon"
        }
        """,
    )

    @Test
    fun `reads the device page the device serves`() {
        val view = parseDeviceSettings(served)
        assertEquals(HealthBadge(DeviceLevel.Blocked, "Mobile data down", "/network#cellular"), view?.health)
        assertEquals(DeviceLevel.Warn, view?.notice?.level)
        assertEquals(DeviceStat("Services", "All running", "3 supervised units", DeviceLevel.Ready), view?.stats?.first())
        assertEquals(listOf(ServiceRow("Video server", "running", DeviceLevel.Ready)), view?.services)
        assertEquals(CloudStage.SigningIn, view?.cloud?.stage)
        assertEquals("ABCD-1234", view?.cloud?.userCode)
        assertEquals("", view?.cloud?.deviceId)
        assertEquals("", view?.cloud?.error)
        assertEquals("Install v0.9.1", view?.software?.candidate?.install)
        assertEquals("E: dpkg", view?.software?.notices?.first()?.detail)
        assertEquals(listOf("Updates come from the Stable channel."), view?.software?.notes)
        assertEquals("falcon", view?.hostname)
    }

    @Test
    fun `an answer without a health line is not a device page`() {
        assertNull(parseDeviceSettings(JSONObject("""{"stats":[]}""")))
    }

    @Test
    fun `missing pieces read as nothing to show, never as a crash`() {
        val view = parseDeviceSettings(JSONObject("""{"health":{"level":0,"text":"All systems operational"}}"""))
        assertEquals("", view?.health?.href)
        assertNull(view?.notice)
        assertEquals(CloudStage.Connect, view?.cloud?.stage)
        assertNull(view?.software?.candidate)
        assertFalse(view?.software?.canCheck ?: true)
    }
}
