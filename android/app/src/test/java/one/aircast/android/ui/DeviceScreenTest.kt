package one.aircast.android.ui

import one.aircast.android.R
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

class DeviceScreenTest {
    private val served = JSONObject(
        """{"name":"falcon","level":2,"headline":"Camera is not sending video","detail":"Check the cable.",
           "action":{"label":"Check the camera","href":"/streaming?camera=cam"},
           "checks":[{"label":"Camera","value":"no video","level":2,"href":"/streaming?camera=cam"}],
           "pages":[{"title":"Ground station links","href":"/connect","icon":"qr-code"},{"title":"Cameras","href":"/streaming","icon":"video","summary":"1 camera, 1 dark","needs":true}]}""",
    )

    private val aircraft = JSONObject(served.toString())
        .put("setup", JSONObject("""{"label":"Finish setting up","href":"/setup"}"""))
        .put("checks", org.json.JSONArray("""[{"label":"Camera","value":"no video","level":2,"href":"/streaming?camera=cam"},{"label":"Telemetry","value":"waiting for autopilot","level":1,"href":"/telemetry"},{"label":"Remote access","value":"not set up","level":1,"href":"/network#tailscale"}]"""))
        .put(
            "pages",
            org.json.JSONArray(
                """[{"title":"Ground station links","href":"/connect","icon":"qr-code"},{"title":"Cameras","href":"/streaming","icon":"video"},
                    {"title":"Video engine","href":"/settings/video","icon":"sliders-horizontal"},{"title":"Connection","href":"/network","icon":"network"},
                    {"title":"Telemetry","href":"/telemetry","icon":"activity"},{"title":"Recordings","href":"/settings/recordings","icon":"gauge"},
                    {"title":"Flight log","href":"/flights","icon":"clock"},{"title":"Device","href":"/settings/device","icon":"settings"},
                    {"title":"Engineering","href":"/settings/engineering","icon":"server"},{"title":"Watching & recording","href":"/settings/camera?camera=cam","icon":"sliders-horizontal"},
                    {"title":"Set up this device step by step","href":"/setup","icon":"scan-search"}]""",
            ),
        )

    @Test
    fun `reads what the device says about itself`() {
        val status = parseDeviceStatus(served)!!
        assertEquals(DeviceLevel.Blocked, status.level)
        assertEquals(DeviceAction("Check the camera", "/streaming?camera=cam"), status.action)
        assertEquals(DeviceCheck("Camera", "no video", 2, "/streaming?camera=cam", ""), status.checks.first())
        assertEquals(listOf(false, true), status.pages.map { it.needs })
        assertTrue(deviceNeedsAttention(status))
        assertEquals("falcon · Camera is not sending video", chipText(status))
        assertFalse(deviceNeedsAttention(parseDeviceStatus(JSONObject(served.toString()).put("level", 0).apply { remove("action") })))
        assertNull(parseDeviceStatus(JSONObject(served.toString()).apply { remove("action") })!!.action)
        assertNull(status.setup)
        assertEquals(DeviceAction("Finish setting up", "/setup"), parseDeviceStatus(aircraft)!!.setup)
        assertNull(parseDeviceStatus(JSONObject("""{"name":"falcon"}""")))
    }

    @Test
    fun `every device page has its own icon, and an unknown one falls back`() {
        assertEquals(R.drawable.ic_videocam, devicePageIcon("video"))
        assertEquals(R.drawable.ic_build, devicePageIcon("scan-search"))
        assertEquals(R.drawable.ic_settings, devicePageIcon("unheard-of"))
    }

    @Test
    fun `every page the device links to, and every page a screen opens, is drawn natively`() {
        val native = { href: String -> devicePath(href).path in DEVICE_SCREENS }
        val status = parseDeviceStatus(aircraft)!!
        val linked = (listOfNotNull(status.action?.href, status.setup?.href) + status.checks.map { it.href } + status.pages.map { it.href }).filter { it.isNotEmpty() }
        assertEquals(16, linked.size)
        assertEquals(emptyList<String>(), linked.filterNot(native))
        val screens = File("src/main/java/one/aircast/android/ui").listFiles { file -> file.name.startsWith("Device") || file.name == "CameraWizard.kt" }.orEmpty()
        val opened = screens.flatMap { file -> Regex("""onOpen\("([^"]+)"\)""").findAll(file.readText()).map { it.groupValues[1] }.toList() }
        assertTrue(opened.size > 5)
        assertEquals(emptyList<String>(), opened.filterNot(native))
    }

    @Test
    fun `a device href splits into the screen it opens and what it asks that screen for`() {
        assertEquals(DevicePath("/settings/camera", mapOf("camera" to "front cam")), devicePath("/settings/camera?camera=front%20cam"))
        assertEquals(DevicePath("/network", emptyMap()), devicePath("/network#tailscale"))
    }

    @Test
    fun `the Device tab is always there, and a requested device page opens on it`() {
        assertEquals(SettingsGroup.Device, SettingsGroup.entries.last())
        assertEquals(SettingsGroup.Device, openingGroup(DEVICE_PAGE))
        assertTrue(pageLook(DEVICE_PAGE).inline)
    }

    @Test
    fun `the device home carries the link and the health the flight sheet showed`() {
        val status = parseDeviceStatus(
            JSONObject(served.toString())
                .put("link", JSONObject("""{"carrier":"Wi-Fi (wlan0)","rate":"↓ 1 kB/s","standby":"Magti · lte","charts":[{"label":"Wi-Fi","values":[60,64],"max":100,"value":"64%","level":3}]}"""))
                .put("health", JSONObject("""{"memory":"400 MB of 1 GB","warning":{"level":1,"text":"Undervoltage seen since start"},"charts":[{"label":"CPU","values":[10,92],"max":100,"value":"92%","level":2}]}"""))
                .put("events", "2 events this flight"),
        )!!
        assertEquals("Wi-Fi (wlan0)", status.link?.carrier)
        assertEquals(DeviceChartView("Wi-Fi", listOf(60.0, 64.0), 100, "64%", 3), status.link?.charts?.first())
        assertEquals(DeviceBadge(1, "Undervoltage seen since start"), status.health?.warning)
        assertEquals(DeviceLevel.Blocked, status.health?.charts?.first()?.level)
        assertEquals("2 events this flight", status.events)
        assertNull(parseDeviceStatus(served)!!.link)
    }
}

class DeviceViewTest {
    @Test
    fun `a device URL is the path itself until a device is connected, and on the device after`() {
        assertEquals("/api/views/aircraft", resolveDeviceUrl("", "/api/views/aircraft"))
        assertEquals("http://192.168.1.100/api/views/aircraft", resolveDeviceUrl("http://192.168.1.100", "/api/views/aircraft"))
        assertEquals("http://falcon.local:8080/api/logs", resolveDeviceUrl("http://falcon.local:8080", "/api/logs"))
    }

    @Test
    fun `the core's reading says which device answered and carries its view`() {
        val reading = deviceReading(JSONObject("""{"origin":"http://10.0.0.5","host":"10.0.0.5","answered":true,"view":{"headline":"Ready"}}"""))
        assertEquals(DeviceReading("http://10.0.0.5", "10.0.0.5", true, reading.view), reading)
        assertEquals("Ready", reading.view?.optString("headline"))
        assertEquals(DeviceReading("", "", false, null), deviceReading(null as JSONObject?))
    }

    @Test
    fun `a refusal names the field the device blamed, and no answer says so`() {
        assertEquals(DeviceRefusal("baud must be a number", "baud"), deviceRefusal(JSONObject("""{"ok":false,"reason":"baud must be a number","field":"baud"}""")))
        assertNull(deviceRefusal(JSONObject("""{"ok":true}""")))
        assertEquals(NO_ANSWER, deviceRefusal(null))
        assertEquals(DeviceAnswer(null, null), deviceAnswer(JSONObject("""{"ok":true,"result":null}""")))
    }

    @Test
    fun `a chart spans its width and clamps values to its scale`() {
        val points = chartPoints(listOf(0.0, 50.0, 150.0), 100, 200f, 40f)
        assertEquals(listOf(0f, 100f, 200f), points.map { it.x })
        assertEquals(listOf(40f, 20f, 0f), points.map { it.y })
        assertEquals(emptyList<Any>(), chartPoints(listOf(5.0), 100, 200f, 40f))
    }

    @Test
    fun `a form sends what was typed over what the device showed, and shows only the fields that apply`() {
        val fields = listOf(
            DeviceField("method", "Addressing", "auto"),
            DeviceField("address", "Address", "", shownWhen = { it["method"] == "manual" }),
        )
        assertEquals(mapOf("method" to "manual", "address" to ""), formBody(fields, mapOf("method" to "manual")))
        assertEquals(listOf("method"), shownFields(fields, formBody(fields, emptyMap())).map { it.name })
        assertEquals(listOf("method", "address"), shownFields(fields, formBody(fields, mapOf("method" to "manual"))).map { it.name })
    }
}
