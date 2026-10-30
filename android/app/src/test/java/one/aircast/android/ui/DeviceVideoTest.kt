package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class DeviceVideoTest {
    private val served =
        """
        {
          "radio": {"status": {"level": 0, "text": "on air"}, "channel": "165", "region": "BO", "txPower": "", "details": "UDP 5602"},
          "sharing": {"status": {"level": 1, "text": "finish setup"}, "stage": "deploy", "deployUrl": "https://deploy.workers.cloudflare.com/?url=x", "signKey": "sk", "deviceToken": "dt"},
          "engineering": {"title": "Engineering", "href": "/settings/engineering", "icon": "server", "summary": "mediamtx"}
        }
        """

    @Test
    fun `reads the video page the device serves`() {
        val view = parseVideo(JSONObject(served))
        assertEquals(RadioSettings(DeviceBadge(0, "on air"), "165", "BO", "", "UDP 5602"), view?.radio)
        assertEquals(SharingStage.Deploy, view?.sharing?.stage)
        assertEquals("sk", view?.sharing?.signKey)
        assertEquals("dt", view?.sharing?.deviceToken)
        assertEquals("", view?.sharing?.baseUrl)
        assertEquals(DevicePageLink("Engineering", "/settings/engineering", "mediamtx"), view?.engineering)
    }

    @Test
    fun `a device with no radio has no radio card`() {
        assertNull(parseVideo(JSONObject(served).apply { remove("radio") })?.radio)
        assertNull(parseVideo(JSONObject().put("radio", JSONObject(served).getJSONObject("radio"))))
    }
}
