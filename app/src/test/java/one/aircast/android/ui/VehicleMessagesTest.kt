package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class VehicleMessagesTest {
    private val real =
        """<font style="<#E>">[18:11:29.402 ] Error: PreArm: Need 3D Fix</font><br/>""" +
            """<font style="<#I>">[18:11:30.100 ] Warning: Low battery</font><br/>""" +
            """<font style="<#N>">[18:17:58.433 ] Info: Frame: QUAD/PLUS</font><br/>"""

    @Test
    fun `the html the vehicle sends becomes readable lines`() {
        assertEquals(
            listOf(
                "[18:11:29.402 ] Error: PreArm: Need 3D Fix",
                "[18:11:30.100 ] Warning: Low battery",
                "[18:17:58.433 ] Info: Frame: QUAD/PLUS",
            ),
            vehicleMessageLines(real),
        )
    }

    @Test
    fun `the colour token inside the style attribute does not leak into the text`() {
        vehicleMessageLines(real).forEach { line ->
            assertEquals("stray markup in: $line", false, line.contains("\"") || line.contains("<"))
        }
    }

    @Test
    fun `severity comes from the untranslated colour token, not the severity word`() {
        assertEquals(
            listOf(MessageSeverity.Error, MessageSeverity.Warning, MessageSeverity.Normal),
            vehicleMessages(real).map { it.severity },
        )
    }

    @Test
    fun `a notice is a warning because QGC groups it that way`() {
        assertEquals(MessageSeverity.Warning, severityOf("""<font style="<#I>">Notice: x</font>"""))
    }

    @Test
    fun `an unmarked message is normal`() {
        assertEquals(MessageSeverity.Normal, severityOf("plain text"))
    }

    @Test
    fun `an empty log yields no lines`() {
        assertEquals(emptyList<String>(), vehicleMessageLines(""))
        assertEquals(emptyList<String>(), vehicleMessageLines("<br/>"))
    }

    @Test
    fun `escaped characters come back as themselves`() {
        assertEquals(
            listOf("""Battery < 20% & "low""""),
            vehicleMessageLines("""<font>Battery &lt; 20% &amp; &quot;low&quot;</font><br/>"""),
        )
    }

    @Test
    fun `an ampersand entity is decoded last so it cannot double-decode`() {
        assertEquals(listOf("&lt;not a tag&gt;"), vehicleMessageLines("&amp;lt;not a tag&amp;gt;<br/>"))
    }

    @Test
    fun `a message without a trailing break is still read`() {
        assertEquals(listOf("Land complete"), vehicleMessageLines("<font>Land complete</font>"))
    }

    @Test
    fun `an armed vehicle is never told what is blocking it`() {
        assertNull(flightBlocker(true, "PreArm: Need 3D Fix", false, true, false))
    }

    @Test
    fun `the vehicle's own prearm text wins over anything I could word`() {
        assertEquals(
            "PreArm: Need 3D Fix",
            flightBlocker(false, "PreArm: Need 3D Fix", false, true, false),
        )
    }

    @Test
    fun `an unhealthy sensor is named when the vehicle gives no prearm text`() {
        assertEquals(
            "A sensor is reporting unhealthy. The vehicle will refuse to arm.",
            flightBlocker(false, "", false, false, false),
        )
    }

    @Test
    fun `not-ready-to-fly only counts when the vehicle signals readiness at all`() {
        assertEquals(
            "The vehicle is not ready to fly yet.",
            flightBlocker(false, "", true, true, false),
        )
        assertNull(flightBlocker(false, "", true, false, false))
    }

    @Test
    fun `a healthy disarmed vehicle shows nothing`() {
        assertNull(flightBlocker(false, "", true, true, true))
    }
}
