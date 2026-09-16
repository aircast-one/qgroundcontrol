package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class RadioLinkTest {

    private fun state(extra: String) = flyState(
        JSONObject(
            """{"kind":"object","class":"FlyState","connected":true,"armed":false,"contactLost":false,
                "state":"ready","stateText":"Ready","staleNotice":"","mode":"Stabilize",
                "rcSupported":true,"rcSignalText":"80%","rcSignal":80,$extra}""",
        ),
    )

    @Test
    fun `a radio that has never spoken is not a link at minus zero`() {
        assertNull(state(""""rcOverride":false,"telemetry":null""")?.telemetry)
        assertNull(telemetryCell(state(""""rcOverride":false,"telemetry":null""")))
        assertEquals(emptyList<DetailRow>(), telemetryDetail(null))
    }

    @Test
    fun `the strip shows this station's own signal`() {
        val reading = state(
            """"rcOverride":false,"telemetry":{"localRssiDbm":-71,"remoteRssiDbm":-68,
               "localNoise":42,"remoteNoise":39,"receiveErrors":7}""",
        )
        assertEquals("-71 dBm", telemetryCell(reading))
        assertEquals(
            listOf("This station", "The vehicle's radio", "Noise here", "Noise at the vehicle", "Packets lost"),
            telemetryDetail(reading?.telemetry).map { it.label },
        )
    }

    @Test
    fun `a radio reporting only its own end still gives a reading and a shorter detail`() {
        val reading = state(
            """"rcOverride":false,"telemetry":{"localRssiDbm":-71,"remoteRssiDbm":null,
               "localNoise":null,"remoteNoise":null,"receiveErrors":null}""",
        )
        assertEquals("-71 dBm", telemetryCell(reading))
        assertEquals(listOf("This station"), telemetryDetail(reading?.telemetry).map { it.label })
    }

    @Test
    fun `the override cell appears only while the transmitter is actually overridden`() {
        assertEquals("RC override", overrideCell(state(""""rcOverride":true,"telemetry":null"""))?.text)
        assertNull(overrideCell(state(""""rcOverride":false,"telemetry":null""")))
        assertNull(
            "null is no vehicle, and claiming manual control is not overridden would be a safety claim from nothing",
            overrideCell(state(""""rcOverride":null,"telemetry":null""")),
        )
    }
}
