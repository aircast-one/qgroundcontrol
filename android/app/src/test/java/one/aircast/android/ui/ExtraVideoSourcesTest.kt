package one.aircast.android.ui

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertFalse
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ExtraVideoSourcesTest {

    private val served = """
        [{"name":"Thermal","source":"RTSP Video Stream","url":"127.0.0.1:8554/thermal","fit":"cover"},
         {"name":"Dome","source":"Video Stream Disabled","url":""}]
    """

    @Test
    fun `sources read back with their name, kind and address`() {
        val sources = extraSources(served)

        assertEquals(listOf("Thermal", "Dome"), sources.map { it.name })
        assertEquals("RTSP Video Stream", sources[0].source)
        assertEquals("127.0.0.1:8554/thermal", sources[0].url)
    }

    @Test
    fun `editing keeps fields this head does not understand`() {
        val next = extraSourcePatched(served, 0, "Thermal 2", "TCP-MPEG2 Video Stream", "10.0.0.4:8100")
        val entry = JSONArray(next).getJSONObject(0)

        assertEquals("Thermal 2", entry.getString("name"))
        assertEquals("10.0.0.4:8100", entry.getString("url"))
        assertEquals("cover", entry.getString("fit"))
    }

    @Test
    fun `adding and removing leave the rest alone`() {
        val added = extraSourceAdded(served, "Wide", "UDP h.264 Video Stream", "0.0.0.0:5601")
        assertEquals(listOf("Thermal", "Dome", "Wide"), extraSources(added).map { it.name })

        val removed = extraSourceRemoved(added, 1)
        assertEquals(listOf("Thermal", "Wide"), extraSources(removed).map { it.name })
        assertEquals("cover", JSONArray(removed).getJSONObject(0).getString("fit"))
    }

    @Test
    fun `a stream that needs an address is not saved without one`() {
        assertEquals("This kind of stream needs an address.",
            extraSourceProblem("RTSP Video Stream", ""))
        assertNull(extraSourceProblem("RTSP Video Stream", "10.0.0.4:8554/front"))
    }

    @Test
    fun `an address carrying its own scheme is refused with the reason`() {
        val problem = extraSourceProblem("TCP-MPEG2 Video Stream", "tcp://10.0.0.4:8100")

        assertTrue(problem!!.contains("scheme"))
        assertNull(extraSourceProblem("TCP-MPEG2 Video Stream", "10.0.0.4:8100"))
    }

    @Test
    fun `a camera with no kind chosen is not saved`() {
        assertEquals("Pick the kind of stream this camera sends.",
            extraSourceProblem("Video Stream Disabled", "anything"))
        assertEquals("Pick the kind of stream this camera sends.", extraSourceProblem("", ""))
    }

    @Test
    fun `the summary says what is missing`() {
        val sources = extraSources(served)

        assertEquals("RTSP Video Stream · 127.0.0.1:8554/thermal", extraSourceSummary(sources[0]))
        assertEquals("Video Stream Disabled", extraSourceSummary(sources[1]))
        assertEquals("RTSP Video Stream · no address",
            extraSourceSummary(ExtraVideoSource("x", "RTSP Video Stream", "")))
    }
}

class VideoKindTest {
    private val raw = listOf("", "Video Stream Disabled", "RTSP Video Stream", "UDP h.264 Video Stream")
    private val cooked = listOf("", "Video Stream Disabled", "Flux RTSP", "Flux UDP h.264")

    @Test
    fun `the chip writes the raw constant, not the label the operator reads`() {
        val kinds = videoKinds(raw, cooked)

        assertEquals(listOf("RTSP Video Stream", "UDP h.264 Video Stream"), kinds.map { it.raw })
        assertEquals(listOf("Flux RTSP", "Flux UDP h.264"), kinds.map { it.label })
    }

    @Test
    fun `the disabled entry and the blank are never offered as a source`() {
        assertEquals(2, videoKinds(raw, cooked).size)
    }

    @Test
    fun `a missing label falls back to the raw name rather than showing nothing`() {
        val kinds = videoKinds(raw, listOf("", "Video Stream Disabled"))

        assertEquals(listOf("RTSP Video Stream", "UDP h.264 Video Stream"), kinds.map { it.label })
    }

    @Test
    fun `a translated list cannot change what gets written`() {
        val translated = listOf("", "Video deshabilitado", "Flujo RTSP", "Flujo UDP")

        assertEquals(videoKinds(raw, cooked).map { it.raw }, videoKinds(raw, translated).map { it.raw })
    }
}

class ExtraSourcesReadingTest {

    private fun view(block: String) = JSONObject("""{"kind":"object","extraSources":$block}""")

    @Test
    fun `a readable list decodes its sources`() {
        val reading = extraSourcesReading(
            view(
                """{"readable":true,"stored":"[]","sources":[
                   {"slot":0,"name":"Nose","source":"RTSP Video Stream","url":"rtsp://x"}]}""",
            ),
        )!!

        assertTrue(reading.readable)
        assertEquals(listOf(ExtraVideoSource("Nose", "RTSP Video Stream", "rtsp://x")), reading.sources)
    }

    @Test
    fun `text that is not a list is not an empty list, and says why`() {
        val broken = extraSourcesReading(
            view(
                """{"readable":false,"stored":"{ not json","sources":[],
                   "reason":"The extra video sources setting is not a readable list."}""",
            ),
        )!!

        assertFalse(broken.readable)
        assertEquals("{ not json", broken.stored)
        assertEquals("The extra video sources setting is not a readable list.", broken.reason)
    }

    @Test
    fun `a view without the block is no reading rather than an empty one`() {
        assertNull(extraSourcesReading(null))
        assertNull(extraSourcesReading(JSONObject("""{"kind":"object"}""")))
    }
}
