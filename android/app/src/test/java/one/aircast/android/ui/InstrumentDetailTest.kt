package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class InstrumentDetailTest {

    private fun row(label: String, value: String, severity: Int = SEVERITY_SECONDARY) =
        """{"label":"$label","value":"$value","severity":$severity}"""

    private fun battery(vararg packs: String, headline: String = "null") = JSONObject(
        """{"kind":"object","class":"Battery","available":true,"packs":[${packs.joinToString(",")}],"headline":$headline}""",
    )

    private fun pack(vararg rows: String) = """{"rows":[${rows.joinToString(",")}]}"""

    @Test
    fun `a battery that is not there has no detail`() {
        assertEquals(emptyList<DetailRow>(), batteryDetail(null))
        assertEquals(
            emptyList<DetailRow>(),
            batteryDetail(JSONObject("""{"kind":"object","available":false,"packs":[]}""")),
        )
        assertEquals(null, batteryHeadline(null))
    }

    @Test
    fun `the core's popup rows are shown in its order with their colouring`() {
        val rows = batteryDetail(battery(pack(row("Time left", "4:10", 1), row("Charge", "30%", 1), row("Voltage", "11.10 V"))))
        assertEquals(listOf("Time left", "Charge", "Voltage"), rows.map { it.label })
        assertEquals(listOf(1, 1, SEVERITY_SECONDARY), rows.map { it.severity })
    }

    @Test
    fun `a second pack is labelled so two voltages cannot be read as one`() {
        val rows = batteryDetail(battery(pack(row("Voltage", "11.10 V")), pack(row("Voltage", "12.30 V"))))
        assertEquals(listOf("Battery 1 Voltage", "Battery 2 Voltage"), rows.map { it.label })
    }

    @Test
    fun `the headline is the worst pack the core picked`() {
        val view = battery(pack(), headline = """{"text":"Critical","detail":"1:30 left","severity":2}""")
        assertEquals(BatteryHeadline("Critical", "1:30 left", 2), batteryHeadline(view))
    }

    @Test
    fun `every placeholder QGC prints for an uncomputed fact is treated as one`() {
        assertTrue(notYetComputed("--.--"))
        assertTrue(notYetComputed("--:--:--"))
        assertTrue(notYetComputed("--"))
        assertTrue(notYetComputed(" "))
        assertFalse(notYetComputed("0"))
        assertFalse(notYetComputed("11.10"))
    }

    @Test
    fun `a dilution the receiver never computed is not a precision claim`() {
        assertFalse(usableDop("--.--"))
        assertFalse(usableDop(""))
        assertFalse(usableDop("0"))
        assertFalse(usableDop("99999"))
        assertTrue(usableDop("1.4"))
    }

    @Test
    fun `gps detail is the lock plus the rows the core served`() {
        val view = JSONObject(
            """{"kind":"object","available":true,"satellites":11,"lock":3,"lockText":"3D Lock","rows":[""" +
                """{"label":"Satellites","value":"11"},{"label":"HDOP","value":"1.4"},{"label":"","value":"x"}]}""",
        )
        val gps = gpsStatus(view)
        assertEquals(11, gps?.satellites)
        assertEquals(FixLevel.Good, fixLevel(gps!!.lock))
        assertEquals(listOf("GPS Lock", "Satellites", "HDOP"), gpsDetail(gps).map { it.label })
        assertEquals("3D Lock", gpsDetail(gps).first().value)
    }

    @Test
    fun `no vehicle and no lock read as nothing rather than a fix`() {
        assertEquals(null, gpsStatus(JSONObject("""{"kind":"object","available":false,"rows":[]}""")))
        val unlocked = gpsStatus(JSONObject("""{"kind":"object","available":true,"satellites":null,"lock":null,"rows":[]}"""))
        assertEquals(null, fixLevel(unlocked!!.lock))
        assertEquals(null, unlocked.satellites)
        assertEquals(emptyList<DetailRow>(), gpsDetail(unlocked))
    }

    @Test
    fun `the lock row shows the core's lock text, so RTK states read as GPSIndicatorPage spells them`() {
        val rtk = gpsStatus(JSONObject("""{"kind":"object","available":true,"satellites":20,"lock":6,"lockText":"RTK Fixed","rows":[]}"""))
        assertEquals(listOf(DetailRow("GPS Lock", "RTK Fixed")), gpsDetail(rtk))
        assertEquals(emptyList<String>(), gpsDetail(null).map { it.label })
    }

    @Test
    fun `the carrying link is named as the one in use`() {
        val links = VehicleLinks(available = true, links = listOf(VehicleLink(false), VehicleLink(true)))
        val rows = linkDetail(links, listOf("Telemetry", "WiFi"), "Telemetry")
        assertEquals(listOf("Telemetry", "WiFi"), rows.map { it.label })
        assertEquals(listOf("carrying", "no contact"), rows.map { it.value })
    }

    @Test
    fun `a link that is neither primary nor lost is standing by, not blank`() {
        val links = VehicleLinks(available = true, links = listOf(VehicleLink(false), VehicleLink(false)))
        assertEquals(
            listOf("carrying", "standing by"),
            linkDetail(links, listOf("Telemetry", "WiFi"), "Telemetry").map { it.value },
        )
    }

    @Test
    fun `the RC sheet reads the same words as the RC cell`() {
        assertEquals(emptyList<DetailRow>(), rcDetail(null))
    }

    @Test
    fun `the status pill hides only when nothing would draw in it`() {
        assertFalse(statusPillShown(vehicle = false, rtk = false, gcsBattery = false))
        assertTrue(statusPillShown(vehicle = false, rtk = false, gcsBattery = true))
        assertTrue(statusPillShown(vehicle = true, rtk = false, gcsBattery = false))
    }
}
