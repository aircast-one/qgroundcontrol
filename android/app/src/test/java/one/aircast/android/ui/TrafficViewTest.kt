package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class TrafficViewTest {

    private fun view(
        enabled: Boolean = true,
        available: Boolean = true,
        connected: Boolean = true,
        receiving: Boolean = true,
        ownPositionKnown: Boolean = true,
        alerting: String = "null",
        alertUnknown: Int = 0,
        emergency: String = "null",
        error: String = "null",
        contacts: String = "[]",
    ) = JSONObject(
        """{"kind":"object","class":"AdsbTraffic","enabled":$enabled,"available":$available,
           "connected":$connected,"receiving":$receiving,"ownPositionKnown":$ownPositionKnown,"alerting":$alerting,"alertUnknown":$alertUnknown,
           "emergency":$emergency,"error":$error,
           "units":{"altitude":"m","velocity":"m/s","heading":"deg","distance":"m","bearing":"deg"},
           "contacts":$contacts}""",
    )

    private val near = """{"icaoAddress":11259375,"callsign":"BAW42","distance":1200.0,
        "bearingDegrees":95.0,"altitude":450.0,"relativeAltitude":200.0,"emergency":null,
        "alert":false,"stale":false}"""

    @Test
    fun `a payload from another view is not a traffic reading`() {
        assertNull(trafficReading(null))
        assertNull(trafficReading(JSONObject("""{"kind":"object","class":"Detections"}""")))
    }

    @Test
    fun `a contact carries the core's converted numbers and the block carries the unit`() {
        val reading = trafficReading(view(contacts = "[$near]"))!!

        assertEquals(1, reading.contacts.size)
        assertEquals("BAW42", reading.contacts[0].name)
        assertEquals("1200 m  95 deg  200 m above", trafficContactText(reading.contacts[0], reading.units))
    }

    @Test
    fun `a contact with no callsign is named by its ICAO address`() {
        val anonymous = near.replace(""""callsign":"BAW42"""", """"callsign":null""")
        val reading = trafficReading(view(contacts = "[$anonymous]"))!!

        assertEquals("ABCDEF", reading.contacts[0].name)
    }

    @Test
    fun `the core withholds range when it cannot place the operator, and zero would be a lie`() {
        val unplaced = near.replace(""""distance":1200.0""", """"distance":null""")
            .replace(""""bearingDegrees":95.0""", """"bearingDegrees":null""")
        val reading = trafficReading(view(contacts = "[$unplaced]"))!!

        assertEquals("bearing unknown  200 m above", trafficContactText(reading.contacts[0], reading.units))
    }

    @Test
    fun `a stale fix says so, because at jet speed it is not where it is drawn`() {
        val old = near.replace(""""stale":false""", """"stale":true""")
        val reading = trafficReading(view(contacts = "[$old]"))!!

        assertEquals("1200 m  95 deg  200 m above  stale", trafficContactText(reading.contacts[0], reading.units))
    }

    @Test
    fun `the server switch does not govern what the vehicle relays over MAVLink`() {
        assertEquals(false, trafficShown(trafficReading(view(enabled = false))!!))
        assertEquals(true, trafficShown(trafficReading(view(enabled = false, contacts = "[$near]"))!!))
        assertEquals(true, trafficShown(trafficReading(view())!!))
    }

    @Test
    fun `a configured receiver switched off is not an absent one`() {
        assertEquals("No traffic receiver", trafficSummary(trafficReading(view(available = false, receiving = false))!!))
        assertEquals(false, trafficReading(view(enabled = false))!!.enabled)
    }

    @Test
    fun `each way of hearing nothing gets its own sentence`() {
        assertEquals("Traffic clear", trafficSummary(trafficReading(view())!!))
        assertEquals("No traffic feed", trafficSummary(trafficReading(view(receiving = false))!!))
        assertEquals(
            "Traffic server unreachable",
            trafficSummary(trafficReading(view(receiving = false, error = """{"token":"connectFailed","detail":"connection refused"}"""))!!),
        )
        assertEquals(
            "Traffic feed dropped",
            trafficSummary(trafficReading(view(receiving = false, error = """{"token":"linkLost","detail":"closed"}"""))!!),
        )
    }

    @Test
    fun `contacts outrank every complaint about the feed, because they are the answer asked for`() {
        val relayed = view(available = false, receiving = false, contacts = "[$near]")

        assertEquals("Traffic: 1 aircraft", trafficSummary(trafficReading(relayed)!!))
    }

    @Test
    fun `an unreported alert is caution, not a calm sky`() {
        assertEquals(TrafficLevel.Good, trafficLevel(trafficReading(view())!!))
        assertEquals(TrafficLevel.Caution, trafficLevel(trafficReading(view(alertUnknown = 1))!!))
        assertEquals(TrafficLevel.Caution, trafficLevel(trafficReading(view(connected = false, receiving = false))!!))
        assertEquals(TrafficLevel.Warning, trafficLevel(trafficReading(view(alerting = "true"))!!))
    }

    @Test
    fun `relayed traffic never states an alert, so its unknown count is not a signal`() {
        val relayed = view(available = false, connected = false, receiving = true, alertUnknown = 1, contacts = "[$near]")

        assertEquals(TrafficLevel.Good, trafficLevel(trafficReading(relayed)!!))
    }

    @Test
    fun `an aircraft squawking an emergency outranks a merely close one`() {
        val panic = view(alerting = "true", emergency = """"hijack"""", contacts = "[$near]")

        assertEquals(TrafficLevel.Critical, trafficLevel(trafficReading(panic)!!))
        assertEquals("squawking hijack", trafficEmergencyText(trafficReading(panic)!!.emergency))
    }

    @Test
    fun `a stated all-clear is not an unknown one`() {
        assertEquals(null, trafficReading(view())!!.alerting)
        assertEquals(false, trafficReading(view(alerting = "false"))!!.alerting)
    }

    @Test
    fun `a coarse unit keeps a decimal, or a close aircraft in miles reads as zero away`() {
        val miles = JSONObject(
            """{"kind":"object","class":"AdsbTraffic","enabled":true,"available":true,"receiving":true,
               "alerting":null,"alertUnknown":0,"emergency":null,"error":null,
               "units":{"altitude":"ft","velocity":"mph","heading":"deg","distance":"mi","bearing":"deg"},
               "contacts":[{"icaoAddress":1,"callsign":"N1","distance":0.4,"bearingDegrees":95.0,
               "altitude":1500.0,"relativeAltitude":200.0,"emergency":null,"alert":false,"stale":false}]}""",
        )
        val reading = trafficReading(miles)!!

        assertEquals("0.4 mi  95 deg  200 ft above", trafficContactText(reading.contacts[0], reading.units))
    }

    @Test
    fun `height is stated against my own, because at my level is the question being asked`() {
        fun at(relative: String) = trafficReading(
            view(contacts = "[${near.replace(""""relativeAltitude":200.0""", """"relativeAltitude":$relative""")}]"),
        )!!

        assertEquals("1200 m  95 deg  200 m above", trafficContactText(at("200.0").contacts[0], at("200.0").units))
        assertEquals("1200 m  95 deg  150 m below", trafficContactText(at("-150.0").contacts[0], at("-150.0").units))
        assertEquals("1200 m  95 deg  my level", trafficContactText(at("0.0").contacts[0], at("0.0").units))
        assertEquals("1200 m  95 deg  450 m", trafficContactText(at("null").contacts[0], at("null").units))
    }

    @Test
    fun `the contact the block is warning about says so on its own row`() {
        val alerting = near.replace(""""alert":false""", """"alert":true""")
        val reading = trafficReading(view(alerting = "true", contacts = "[$alerting]"))!!

        assertEquals("1200 m  95 deg  200 m above  alerting", trafficContactText(reading.contacts[0], reading.units))
    }

    @Test
    fun `the reference for every number is stated once, not guessed from a row`() {
        assertEquals(
            "Range, bearing and height are relative to the vehicle",
            trafficCaption(trafficReading(view())!!),
        )
        assertEquals(
            "No vehicle position, so nothing can be ranged",
            trafficCaption(trafficReading(view(ownPositionKnown = false))!!),
        )
    }

    @Test
    fun `the row that earned the warning is the one marked, not the whole list`() {
        val calm = trafficReading(view(contacts = "[$near]"))!!.contacts[0]
        val loud = trafficReading(view(contacts = "[${near.replace(""""alert":false""", """"alert":true""")}]"))!!.contacts[0]
        val squawking = trafficReading(view(contacts = "[${near.replace(""""emergency":null""", """"emergency":"hijack"""")}]"))!!.contacts[0]

        assertEquals(false, trafficContactUrgent(calm))
        assertEquals(true, trafficContactUrgent(loud))
        assertEquals(true, trafficContactUrgent(squawking))
    }

    @Test
    fun `an absolute altitude says what it is measured from`() {
        val units = TrafficUnits(distance = "ft", altitude = "ft", heading = "deg")
        val contact = { type: String ->
            TrafficContact(
                icaoAddress = 1, callsign = "SWR000", distance = null, bearingDegrees = null,
                altitude = 1000.0, relativeAltitude = null, emergency = "", alert = null,
                stale = false, altitudeType = type,
            )
        }
        assertEquals(
            "with no vehicle there is nothing to be relative TO, so the bare number is the " +
                "aircraft's own altitude - and 1000 ft reads as a separation from a reader who " +
                "has just been shown '500 ft above' for the same slot",
            "1000 ft by pressure",
            trafficHeightText(contact("pressureQnh"), units),
        )
        assertEquals("1000 ft by GPS", trafficHeightText(contact("geometric"), units))
        assertEquals(
            "a datum this head does not know is left unnamed rather than guessed",
            "1000 ft",
            trafficHeightText(contact("somethingNew"), units),
        )
    }

    @Test
    fun `a height relative to the vehicle still reads as a separation`() {
        val units = TrafficUnits(distance = "ft", altitude = "ft", heading = "deg")
        val above = TrafficContact(
            icaoAddress = 1, callsign = "A", distance = 100.0, bearingDegrees = 90.0,
            altitude = 1000.0, relativeAltitude = 500.0, emergency = "", alert = null,
            stale = false, altitudeType = "pressureQnh",
        )
        assertEquals(
            "the datum belongs only on the absolute form; 500 ft above is already relative to you",
            "500 ft above",
            trafficHeightText(above, units),
        )
    }
}
