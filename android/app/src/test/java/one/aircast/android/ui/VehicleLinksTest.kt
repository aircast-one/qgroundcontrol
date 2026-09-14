package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class VehicleLinksTest {

    private fun view(
        available: Boolean = true,
        watching: Boolean = true,
        primary: String = "\"LTE\"",
        contactLost: String = "false",
        links: String = """[{"name":"LTE","primary":true,"commLost":false},
                            {"name":"radio","primary":false,"commLost":false}]""",
    ) = JSONObject(
        """{"kind":"object","class":"VehicleLinks","available":$available,"watching":$watching,
           "primary":$primary,"contactLost":$contactLost,"reason":"","links":$links}""",
    )

    @Test
    fun `another view is not a links reading`() {
        assertNull(vehicleLinks(null))
        assertNull(vehicleLinks(JSONObject("""{"kind":"object","class":"AdsbTraffic"}""")))
    }

    @Test
    fun `one link is the ordinary case and says nothing`() {
        val single = view(links = """[{"name":"LTE","primary":true,"commLost":false}]""")

        assertNull(linkCell(vehicleLinks(single)))
    }

    @Test
    fun `two healthy links say so, because redundancy is the thing being reported`() {
        assertEquals(LinkCell("2 links", false), linkCell(vehicleLinks(view())))
    }

    @Test
    fun `a silent radio is counted, not named - the strip has no room for a link name`() {
        val degraded = view(
            links = """[{"name":"LTE","primary":true,"commLost":false},
                        {"name":"radio","primary":false,"commLost":true}]""",
        )

        assertEquals(LinkCell("1 link lost", true), linkCell(vehicleLinks(degraded)))
    }

    @Test
    fun `every link silent is a different sentence from one of them silent`() {
        val gone = view(
            contactLost = "true",
            links = """[{"name":"LTE","primary":true,"commLost":true},
                        {"name":"radio","primary":false,"commLost":true}]""",
        )

        assertEquals(LinkCell("no link heard", true), linkCell(vehicleLinks(gone)))
    }

    @Test
    fun `an unwatched vehicle reports no loss, and a null is not a false`() {
        val unwatched = view(
            watching = false,
            contactLost = "null",
            links = """[{"name":"LTE","primary":true,"commLost":null},
                        {"name":"radio","primary":false,"commLost":null}]""",
        )
        val reading = vehicleLinks(unwatched)!!

        assertNull(reading.contactLost)
        assertNull(reading.links[0].commLost)
        assertEquals(LinkCell("2 links", false), linkCell(reading))
    }

    @Test
    fun `no vehicle is no cell`() {
        assertNull(linkCell(vehicleLinks(view(available = false, links = "[]"))))
    }

    @Test
    fun `two of three silent is not one of them`() {
        val worse = view(
            links = """[{"name":"LTE","primary":true,"commLost":false},
                        {"name":"radio","primary":false,"commLost":true},
                        {"name":"spare","primary":false,"commLost":true}]""",
        )

        assertEquals(LinkCell("2 links lost", true), linkCell(vehicleLinks(worse)))
    }
}
