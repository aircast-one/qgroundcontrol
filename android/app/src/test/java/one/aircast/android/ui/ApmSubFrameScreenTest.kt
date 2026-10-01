package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ApmSubFrameScreenTest {
    @Test
    fun `frames, the selection and the confirm rule come from the core`() {
        val read = subFrames(JSONObject("""{"available":true,"selected":1,"confirmFirst":true,"loadingDefaults":false,"loadError":"",
            "frames":[{"name":"BlueROV1","value":0,"hasDefaults":false},{"name":"BlueROV2/Vectored","value":1,"hasDefaults":true}]}"""))!!
        assertEquals(listOf("BlueROV1", "BlueROV2/Vectored"), read.frames.map { it.name })
        assertEquals(1, read.selected)
        assertTrue(read.confirmFirst)
        assertTrue(read.frames[1].hasDefaults)
        assertNull(subFrames(JSONObject("""{"available":false}""")))
    }
}
