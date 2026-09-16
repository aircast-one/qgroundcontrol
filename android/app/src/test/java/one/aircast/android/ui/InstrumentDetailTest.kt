package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class InstrumentDetailTest {

    private fun fact(name: String, value: String, units: String = "") =
        """{"name":"$name","units":"$units","valueString":"$value"}"""

    private fun battery(vararg packs: String) = JSONObject(
        """{"kind":"object","class":"Battery","available":true,"packs":[${packs.joinToString(",")}]}""",
    )

    private fun pack(charge: String, vararg facts: String) =
        """{"chargeLabel":"$charge","facts":[${facts.joinToString(",")}]}"""

    @Test
    fun `a battery that is not there has no detail`() {
        assertEquals(emptyList<DetailRow>(), batteryDetail(null))
        assertEquals(
            emptyList<DetailRow>(),
            batteryDetail(JSONObject("""{"kind":"object","available":false,"packs":[]}""")),
        )
    }

    @Test
    fun `every reported fact is named and carries its units`() {
        val rows = batteryDetail(battery(pack("n/a", fact("voltage", "11.10", "v"), fact("mahConsumed", "1800", "mAh"))))
        assertEquals(listOf("Voltage", "Consumed"), rows.map { it.label })
        assertEquals(listOf("11.10 v", "1800 mAh"), rows.map { it.value })
    }

    @Test
    fun `a fact the vehicle has not filled in is left out rather than shown blank`() {
        val rows = batteryDetail(battery(pack("n/a", fact("voltage", "11.10", "v"), fact("temperature", "--.--", "F"))))
        assertEquals(listOf("Voltage"), rows.map { it.label })
    }

    @Test
    fun `every placeholder QGC prints for an uncomputed fact is treated as one`() {
        assertTrue(notYetComputed("--.--"))
        assertTrue(notYetComputed("--:--:--"))
        assertTrue(notYetComputed("--"))
        assertTrue(notYetComputed(" "))
        assertFalse(notYetComputed("0"))
        assertFalse(notYetComputed("11.10"))
        val rows = batteryDetail(
            battery(pack("n/a", fact("voltage", "11.10", "v"), fact("timeRemainingStr", "--:--:--"))),
        )
        assertEquals(listOf("Voltage"), rows.map { it.label })
    }

    @Test
    fun `a second pack is labelled so two voltages cannot be read as one`() {
        val rows = batteryDetail(
            battery(pack("n/a", fact("voltage", "11.10", "v")), pack("n/a", fact("voltage", "12.30", "v"))),
        )
        assertEquals(listOf("Battery 1 Voltage", "Battery 2 Voltage"), rows.map { it.label })
    }

    @Test
    fun `a charge state is shown only when the vehicle names one`() {
        assertEquals(
            listOf("Voltage"),
            batteryDetail(battery(pack("n/a", fact("voltage", "11.10", "v")))).map { it.label },
        )
        assertEquals(
            listOf("Voltage", "State"),
            batteryDetail(battery(pack("Charging", fact("voltage", "11.10", "v")))).map { it.label },
        )
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
    fun `gps detail drops what the receiver has not answered`() {
        val rows = gpsDetail(count = "11", fix = FixLevel.Good, hdop = "1.4", vdop = "--.--", course = "--.--")
        assertEquals(listOf("GPS lock", "Satellites", "HDOP"), rows.map { it.label })
        assertEquals(
            "165.12°",
            gpsDetail(count = "", fix = null, hdop = "", vdop = "", course = "165.12").single().value,
        )
    }

    @Test
    fun `the lock is named rather than shown as the enum the vehicle sent`() {
        assertEquals("3D or better", lockName(FixLevel.Good))
        assertEquals("2D", lockName(FixLevel.TwoD))
        assertEquals("No fix", lockName(FixLevel.None))
        assertEquals(null, lockName(null))
        assertEquals(
            emptyList<String>(),
            gpsDetail(count = "", fix = null, hdop = "", vdop = "", course = "").map { it.label },
        )
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
}
