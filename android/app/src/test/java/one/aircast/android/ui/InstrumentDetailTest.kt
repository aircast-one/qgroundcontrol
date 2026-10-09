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
    fun `two packs read as one line each, with the limiting one marked`() {
        val view = battery(
            pack(row("Charge", "90%"), row("Voltage", "12.60 V")),
            pack(row("Charge", "79%", 1), row("Voltage", "12.30 V")),
            headline = """{"text":"79%","detail":"","severity":0,"index":1}""",
        )
        assertEquals(
            listOf(DetailRow("Battery 1", "90% \u00b7 12.60 V", 0), DetailRow("Battery 2 \u00b7 lowest", "79% \u00b7 12.30 V", 1)),
            batteryDetail(view),
        )
    }

    @Test
    fun `the headline is the limiting pack the core picked, with its level and failsafe margin`() {
        val view = battery(pack(), headline = """{"text":"Critical","detail":"1:30 left","severity":2,"level":"critical","margin":"Returns home at 7%","index":0}""")
        assertEquals(BatteryHeadline("Critical", "1:30 left", 2, BatteryLevel.Critical, "Returns home at 7%", 0), batteryHeadline(view))
    }

    @Test
    fun `the bar turns amber at the turn-back point and red in the reserve`() {
        val view = battery(pack(), headline = """{"text":"41%","detail":"","severity":0,"percent":41.0,"timeLeft":"6:09","reserve":7.0,"returnAt":26.0,"returnNow":false}""")
        val calm = batteryHeadline(view)!!
        assertEquals(Triple(41.0, "6:09", 26.0), Triple(calm.percent, calm.timeLeft, calm.returnAt))
        assertEquals(BarTone.Fine, barTone(calm))
        assertEquals(BarTone.ReturnNow, barTone(calm.copy(percent = 25.0, returnNow = true)))
        assertEquals(BarTone.Reserve, barTone(calm.copy(percent = 6.0, returnNow = true)))
        assertEquals("Battery 41%, return home needed at 26%", batteryBarDescription(calm))
    }

    @Test
    fun `return now is said once a flight, however often the charge crosses the turn-back point`() {
        val first = returnAlert(alerted = false, returnNow = true, flying = true)
        assertEquals(ReturnAlert(speak = true, alerted = true), first)
        val wobble = returnAlert(returnAlert(first.alerted, returnNow = false, flying = true).alerted, returnNow = true, flying = true)
        assertEquals(ReturnAlert(speak = false, alerted = true), wobble)
        assertEquals("landing re-arms it for the next flight", false, returnAlert(true, returnNow = false, flying = false).alerted)
    }

    @Test
    fun `silence reads in seconds, then minutes`() {
        assertEquals("No data from the aircraft for 12 s", silenceText(12))
        assertEquals("No data from the aircraft for 2 min", silenceText(150))
    }

    @Test
    fun `the vehicle chip says the signal is lost and for how long`() {
        assertEquals("Signal lost \u00b7 12 s", signalLostTitle(12))
        assertEquals("Signal lost", osdModeText(signalLostTitle(12)))
        assertEquals("12 s", osdStatusNote(signalLostTitle(12)))
        assertEquals("Signal lost", signalLostTitle(null))
    }

    @Test
    fun `the chip counts down to the lost-link failsafe, then names it`() {
        val home = LossFailsafe("Return home", 10.0)
        assertEquals("Signal lost \u00b7 7 s \u00b7 Return home in 3 s", signalLostTitle(7, home))
        assertEquals("Signal lost \u00b7 12 s \u00b7 Return home", signalLostTitle(12, home))
        assertEquals("Signal lost \u00b7 7 s \u00b7 No failsafe", signalLostTitle(7, LossFailsafe("No failsafe", null)))
        assertEquals("the portrait chip keeps the countdown and shortens the rest", "Lost 2 s \u00b7 Return home in 8 s", signalLostTitle(2, home, compact = true))
    }

    @Test
    fun `the core's spoken lines are read in order after the last one heard`() {
        val batch = speechBatch(JSONObject("""{"class":"Speech","last":4,"lines":[{"sequence":3,"text":"communication lost","volume":0.5},{"sequence":4,"text":"armed","volume":1}]}"""))
        assertEquals(SpeechBatch(4, false, listOf(SpokenLine(3, "communication lost", 0.5f), SpokenLine(4, "armed", 1f))), batch)
        assertEquals("a muted core silences the return-now line too, as QGC's AudioOutput mutes every line", true, speechBatch(JSONObject("""{"class":"Speech","last":4,"muted":true,"lines":[]}"""))?.muted)
        assertEquals("view.speech(4)", speechPath(4))
        assertEquals(null, speechBatch(null))
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
