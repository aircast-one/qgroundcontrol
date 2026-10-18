package one.aircast.android.ui

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class DeviceCamerasTest {
    private val wizard = """{"name":"","new":true,"suggestedName":"cam","interfaces":[],"transports":[],"others":[],"templates":[],"subnets":[]}"""

    private fun camera(name: String, live: Boolean): JSONObject = JSONObject(
        """
        {
          "name": "$name",
          "status": {"level": ${if (live) 0 else 2}, "text": "${if (live) "live" else "unreachable"}"},
          "live": $live,
          "preview": "${if (live) "/whep/$name" else ""}",
          "placeholder": "${if (live) "" else "Camera offline"}",
          "watch": "/watch/$name",
          "copies": [{"label": "Copy watch link", "text": "http://dev/watch/$name", "qr": false}],
          "facts": [{"label": "Source", "value": "rtsp://10.0.0.1/s"}],
          "bitrate": {"label": "Bitrate", "values": [1, 2], "max": 2, "value": "16 bps", "level": 3},
          "links": [{"proto": "RTSP", "label": "QGroundControl", "url": "rtsp://dev:8554/$name"}],
          "chips": [{"key": "cloud", "label": "cloud connected", "level": 0, "href": "/settings/camera?camera=$name#cam-watch"}],
          "wizard": {"name": "$name", "new": false, "suggestedName": "cam", "interfaces": [], "transports": [], "others": [], "templates": [], "subnets": []},
          "setup": {
            "watch": {
              "part": {"title": "Where to watch", "summary": "Ground station, browser, QR codes.", "badge": {"level": 1, "text": "cloud sign-in"}, "open": true},
              "cloud": true,
              "signIn": true,
              "cloudLinks": [],
              "links": []
            },
            "radio": {
              "part": {"title": "Video without cellular", "summary": "s"},
              "notice": {"level": 3, "text": "needs the engine", "link": {"label": "What carries video", "href": "/settings/video"}},
              "form": false
            },
            "servers": {
              "part": {"title": "Your own server", "summary": "s"},
              "dests": [{"kind": "cloudflare", "part": {"title": "Cloudflare Realtime", "summary": "s"}, "on": true, "app": "a", "track": "t", "watch": "/watch/$name?via=cloudflare"}]
            },
            "shares": {
              "part": {"title": "Share links", "summary": "s"},
              "links": [{"id": "x", "label": "—", "url": "http://dev/s/x", "dead": "Turned off 1 Jan 09:00"}],
              "hours": [{"value": "24", "label": "1 day"}]
            },
            "detect": {"part": {"title": "Detection overlay", "summary": "s"}, "on": false},
            "record": {"part": {"title": "Recording", "summary": "s"}, "on": true, "full": true}
          },
          "topic": {"title": "Watching & recording", "href": "/settings/camera?camera=$name", "icon": "sliders-horizontal", "summary": "where it goes"}
        }
        """,
    )
        .put("diagnosis", if (live) null else JSONObject("""{"level": 2, "text": "Camera not responding"}"""))
        .put("fix", if (live) null else JSONObject("""{"label": "Scan for it", "scan": true}"""))

    private val served = JSONObject()
        .put("cameras", JSONArray(listOf(camera("cam2", true), camera("cam3", false))))
        .put("add", JSONObject(wizard))
        .put("cloudLink", "https://app/x")
        .put("video", JSONObject("""{"title": "Video engine", "href": "/settings/video", "icon": "sliders-horizontal"}"""))

    private fun cameras(): List<CameraView> = parseCameras(served)?.cameras.orEmpty()

    @Test
    fun `reads the cameras view the device serves`() {
        val view = parseCameras(served)
        assertEquals(listOf("cam2", "cam3"), view?.cameras?.map { it.name })
        assertEquals("https://app/x", view?.cloudLink)
        assertEquals(true, view?.add?.isNew)
        val (live, dark) = view?.cameras.orEmpty()
        assertEquals(DeviceBadge(DeviceLevel.Ready, "live"), live.status)
        assertEquals("/whep/cam2", live.preview)
        assertEquals(2, live.bitrate?.max)
        assertNull(live.diagnosis)
        assertEquals(CameraFix("Scan for it", true), dark.fix)
        assertEquals(DeviceLevel.Blocked, dark.diagnosis?.level)
        assertEquals(DeviceBadge(DeviceLevel.Warn, "cloud sign-in"), live.setup.watch.part.badge)
        assertEquals(NoticeLink("What carries video", "/settings/video"), live.setup.radio.notice.link)
        val dest = live.setup.servers.dests.first()
        assertEquals(listOf("cloudflare", "a", "t", "/watch/cam2?via=cloudflare"), listOf(dest.kind, dest.app, dest.track, dest.watch))
        assertTrue(dest.on)
        assertEquals("Turned off 1 Jan 09:00", live.setup.shares.links.first().dead)
        assertNull(live.setup.detect.notice)
        assertEquals(listOf(true, true), listOf(live.setup.record.on, live.setup.record.full))
        assertEquals("", live.setup.record.usage)
        assertNull(live.setup.gimbal)
        assertEquals("/settings/camera?camera=cam2", live.topic.href)
        assertNull(parseCameras(JSONObject("""{"cameras": []}""")))
    }

    @Test
    fun `opens the only camera, walls several, and adds when asked or when there are none`() {
        val all = cameras()
        val one = all.take(1)
        assertEquals("cam2", (camerasShown(one, "", false) as? CamerasShown.Camera)?.camera?.name)
        assertEquals(CamerasShown.Wall, camerasShown(all, "", false))
        assertEquals("cam3", (camerasShown(all, "cam3", false) as? CamerasShown.Camera)?.camera?.name)
        assertEquals(CamerasShown.Wall, camerasShown(all, "gone", false))
        assertEquals(CamerasShown.Add, camerasShown(all, "cam3", true))
        assertEquals(CamerasShown.Add, camerasShown(emptyList(), "cam2", false))
    }

    @Test
    fun `announces a camera only when it comes back from dark`() {
        val before = liveness(cameras())
        assertEquals(false, before["cam3"])
        val back = cameras().map { it.copy(live = true) }
        assertEquals(listOf("cam3"), cameBack(before, back))
        assertEquals(emptyList<String>(), cameBack(emptyMap(), back))
        assertEquals(emptyList<String>(), cameBack(liveness(back), back))
    }

    @Test
    fun `composes the Cloudflare target from the app and track`() {
        assertEquals("cloudflare://app-123/cam2-track", cloudflareTarget(" app-123 ", "cam2-track"))
        assertEquals("", cloudflareTarget("app-123", ""))
    }

    @Test
    fun `a chip opens the setting it names`() {
        assertEquals("cam-watch", openedSection("/settings/camera?camera=cam2#cam-watch"))
        assertEquals("", openedSection("/settings/camera?camera=cam2"))
    }
}
