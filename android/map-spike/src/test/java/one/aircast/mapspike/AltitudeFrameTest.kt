package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class AltitudeFrameTest {

    private fun item(
        frameText: String,
        text: String = "50.0 m",
        band: String = "",
        editUnits: String = "",
    ) = MissionItem(
        2, 2, 41.0, 44.0, "Waypoint", false, 50.0, kind = "waypoint",
        altitudeText = text, altitudeBandText = band, altitudeFrameText = frameText,
        altitudeEditUnits = editUnits,
    )

    @Test
    fun `a height above launch is the one an operator already assumes, so it is left bare`() {
        assertEquals("50.0 m", altitudeWithFrame(item("")))
    }

    @Test
    fun `the two frames that are not the default say so`() {
        assertEquals("541 m AMSL", altitudeWithFrame(item("AMSL", "541 m")))
        assertEquals("50.0 m AGL", altitudeWithFrame(item("AGL")))
    }

    @Test
    fun `a survey and a takeoff both reading fifty metres are no longer the same row`() {
        val takeoff = item("")
        val survey = item("AGL")

        assertEquals("50.0 m", itemDetail(takeoff))
        assertEquals("50.0 m AGL", itemDetail(survey))
    }

    @Test
    fun `a band takes the frame too, rather than only single heights`() {
        assertEquals(
            "0.0 m to 40.0 m AMSL",
            altitudeWithFrame(item("AMSL", text = "", band = "0.0 m to 40.0 m")),
        )
    }

    @Test
    fun `a frame the core withheld leaves the height unlabelled rather than guessing one`() {
        assertEquals("50.0 m", altitudeWithFrame(item("")))
        assertNull(altitudeWithFrame(item("AMSL", text = "")))
    }

    @Test
    fun `the summary chip carries the frame too, being the other place a height is spelled`() {
        val amsl = item("AMSL", "541 m")

        assertEquals("#2 at 541 m AMSL", selectionText(MapHit.Waypoint(2), listOf(amsl), emptyList(), emptyList()))
        assertEquals("#2 at 50.0 m", selectionText(MapHit.Waypoint(2), listOf(item("")), emptyList(), emptyList()))
    }

    @Test
    fun `the field being typed into names its frame, being the furthest thing from the chip`() {
        assertEquals("Alt m", altitudeFieldLabel(item("")))
        assertEquals("Alt m AMSL", altitudeFieldLabel(item("AMSL")))
        assertEquals("Alt m AGL", altitudeFieldLabel(item("AGL")))
        assertEquals("Alt m SEABED", altitudeFieldLabel(item("SEABED")))
    }

    @Test
    fun `a frame neither head has heard of is shown, not swallowed into the default`() {
        assertEquals("50.0 m SEABED", altitudeWithFrame(item("SEABED")))
    }

    @Test
    fun `the field being typed into names the unit QGC cooked the number into`() {
        assertEquals("Alt ft", altitudeFieldLabel(item("", editUnits = "ft")))
        assertEquals("Alt ft AMSL", altitudeFieldLabel(item("AMSL", editUnits = "ft")))
    }

    @Test
    fun `a core that says nothing leaves metres, which is what the bridge takes`() {
        assertEquals("Alt m", altitudeFieldLabel(item("")))
    }
}
