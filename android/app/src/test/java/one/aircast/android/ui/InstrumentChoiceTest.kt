package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class InstrumentChoiceTest {
    @Test
    fun `the tiles already shown lead the sheet in their own order and leave their groups`() {
        val alt = InstrumentFact("alt", "Altitude", "a.alt")
        val speed = InstrumentFact("speed", "Speed", "a.speed")
        val roll = InstrumentFact("roll", "Roll", "v.roll")
        val groups = listOf(InstrumentGroup("vehicle", "Vehicle", listOf(roll)), InstrumentGroup("air", "Air", listOf(alt, speed)))

        assertEquals(
            listOf("Shown" to listOf("Speed", "Altitude"), "Vehicle" to listOf("Roll")),
            shownFirst(groups, listOf("a.speed", "a.alt")).map { it.title to it.facts.map(InstrumentFact::label) },
        )
        assertEquals(listOf("Vehicle", "Air"), shownFirst(groups, emptyList()).map { it.title })
    }


    private val served = JSONObject(
        """
        {"available":true,"groups":[
          {"group":"gps","title":"GPS","facts":[
            {"name":"lat","label":"Latitude","selection":"gps/lat"},
            {"name":"hdop","label":"HDOP","selection":"gps/hdop"}]},
          {"group":"wind","title":"Wind","facts":[
            {"name":"speed","label":"Wind Speed","selection":"wind/speed"}]},
          {"group":"empty","title":"Empty","facts":[]}
        ]}
        """,
    )

    @Test
    fun `a group the vehicle reports nothing for is not offered`() {
        assertEquals(listOf("gps", "wind"), instrumentGroups(served).map { it.group })
    }

    @Test
    fun `a fact is asked for by the selection the core spells`() {
        val gps = instrumentGroups(served).first()
        assertEquals(
            "the head built this string itself until 7273d35a6 and got it wrong once already - a " +
                "bare lon resolves against the vehicle group and answers noSuchFact, which this " +
                "head drops silently, so the reading an operator picked never appeared",
            listOf("gps/lat", "gps/hdop"),
            gps.facts.map { it.path },
        )
        assertEquals("Latitude", gps.facts.first().label)
    }

    @Test
    fun `a fact the core did not spell a selection for is not offered`() {
        val unspelled = JSONObject(
            """{"available":true,"groups":[{"group":"gps","title":"GPS","facts":[
                 {"name":"lat","label":"Latitude"}]}]}""",
        )
        assertTrue(
            "a row that cannot be asked for is a row that silently does nothing when tapped",
            instrumentGroups(unspelled).isEmpty(),
        )
    }

    @Test
    fun `no vehicle offers nothing rather than an empty catalogue`() {
        assertTrue(instrumentGroups(JSONObject("""{"available":false,"groups":[]}""")).isEmpty())
        assertTrue(instrumentGroups(null).isEmpty())
    }

    @Test
    fun `choosing nothing draws nothing, and the core's own fallback is never reached`() {
        assertEquals("view.instruments(lat,hdop)", instrumentsPath(listOf("lat", "hdop")))
        assertEquals("the default selection asks the core, which adds airspeed for a wing as QGCCorePlugin does", "view.instruments", instrumentsPath(DEFAULT_INSTRUMENTS))
        assertEquals("a wing's default list with airspeed still asks the core, keeping the AirSpd text", "view.instruments", instrumentsPath(defaultInstruments("fixedWing"), "fixedWing"))
        assertEquals("a wing with airspeed turned off names its list, so the core does not add it back", "view.instruments(${DEFAULT_INSTRUMENTS.joinToString(",")})", instrumentsPath(DEFAULT_INSTRUMENTS, "fixedWing"))
        assertTrue(showsInstruments(listOf("lat")))
        assertFalse(
            "instruments_view answers its OWN four defaults for an empty argument list, so a head " +
                "that stopped naming readings would get four back while the sheet drew them as " +
                "unchecked - the strip is gated here rather than by the path",
            showsInstruments(emptyList()),
        )
    }

    @Test
    fun `choosing toggles with no limit, the row wraps like TelemetryChipsLayer`() {
        assertEquals(listOf("a", "b"), withInstrument(listOf("a"), "b"))
        assertEquals(listOf("a"), withInstrument(listOf("a", "b"), "b"))
        val many = (1..9).map { "f$it" }
        assertEquals(many + "extra", withInstrument(many, "extra"))
    }

    @Test
    fun `the note says where the operator stands`() {
        assertTrue(instrumentChoiceNote(emptyList()).contains("no readings"))
        assertEquals("2 chosen.", instrumentChoiceNote(listOf("a", "b")))
    }

    @Test
    fun `the readings the screen starts with are in the catalogue and match what is chosen`() {
        val own = vehicleOwnGroup(
            JSONObject(
                """{"kind":"object","available":true,"groups":[],"vehicleFacts":[
                     {"name":"altitudeRelative","label":"Alt (Rel)","selection":"altitudeRelative"},
                     {"name":"groundSpeed","label":"Ground Speed","selection":"groundSpeed"},
                     {"name":"distanceToHome","label":"Distance to Home","selection":"distanceToHome"},
                     {"name":"heading","label":"Heading","selection":"heading"},
                     {"name":"climbRate","label":"Climb Rate","selection":"climbRate"},
                     {"name":"rangeFinderDist","label":"Range Finder Dist","selection":"rangeFinderDist"}]}""",
            ),
        )!!
        assertEquals("Vehicle", own.title)
        assertTrue(
            "view.instrumentGroups serves the vehicle's own readings apart from its child groups, so " +
                "without this the four defaults are absent from the sheet and cannot be unchecked",
            DEFAULT_INSTRUMENTS.all { name -> own.facts.any { it.path == name } },
        )
        assertEquals(
            "a top-level fact is asked for by bare name; only a child group's fact is qualified",
            "altitudeRelative",
            own.facts.first().path,
        )
        assertEquals(
            "five of the twenty-eight carry no description; view.instrumentGroups humanises those",
            "Range Finder Dist",
            own.facts.last().label,
        )
    }

    @Test
    fun `no vehicle contributes no group rather than an empty one`() {
        assertNull(vehicleOwnGroup(JSONObject("""{"kind":"object","available":false,"vehicleFacts":[]}""")))
        assertNull(vehicleOwnGroup(JSONObject("""{"kind":"object","available":true,"vehicleFacts":[]}""")))
        assertNull(vehicleOwnGroup(null))
    }

    @Test
    fun `an empty catalogue with a vehicle connected does not tell the operator to connect one`() {
        assertEquals(
            "every fact is dropped when none carries a selection, which is what an older library " +
                "looks like - and a connect prompt in front of a connected vehicle reads as a " +
                "regression rather than as the head refusing to guess",
            "This vehicle reported no readings this screen can ask for.",
            emptyCatalogueText(connected = true),
        )
        assertEquals("Connect a vehicle to see what it can report.", emptyCatalogueText(connected = false))
    }

    @Test
    fun `each vehicle class keeps its own instruments, like FactValueGrid's settings key`() {
        assertEquals("multiRotor", instrumentVehicleClass(org.json.JSONObject("""{"vehicleClass":"multiRotor"}""")))
        assertEquals("generic", instrumentVehicleClass(null))
        assertEquals("chosen-fixedWing", chosenKey("fixedWing"))
    }

    @Test
    fun `the size pill cycles QGC's four value sizes and says which`() {
        assertEquals(listOf("Default", "Small", "Medium", "Large"), ValueSize.entries.map { it.label })
        assertEquals(ValueSize.Small, nextValueSize(ValueSize.Default))
        assertEquals(ValueSize.Default, nextValueSize(ValueSize.Large))
        assertEquals(ValueSize.Default, valueSizeAt(9))
        assertEquals("Size: Medium", valueSizePillText(ValueSize.Medium))
        assertEquals(listOf(1f, 0.86f, 1.25f, 1.5f), ValueSize.entries.map { it.scale })
    }

    @Test
    fun `a scaled value keeps the telemetry figures and grows its line with it`() {
        val large = scaledNumber(1.5f)
        assertEquals(one.aircast.mapspike.TelemetryNumber.fontSize * 1.5f, large.fontSize)
        assertEquals(one.aircast.mapspike.TelemetryNumber.fontFeatureSettings, large.fontFeatureSettings)
    }

    @Test
    fun `values move, swap their reading and go away in place`() {
        val row = listOf("a", "b", "c")
        assertEquals(listOf("b", "a", "c"), movedInstrument(row, 1, -1))
        assertEquals(row, movedInstrument(row, 2, 1))
        assertEquals(listOf("a", "x", "c"), replacedInstrument(row, 1, "x"))
        assertEquals("a reading already in the row moves rather than doubling", listOf("c", "b"), replacedInstrument(row, 0, "c"))
        assertEquals(listOf("a", "c"), removedInstrument(row, 1))
    }
}
