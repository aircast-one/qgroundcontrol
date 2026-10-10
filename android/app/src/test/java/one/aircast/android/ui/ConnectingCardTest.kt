package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class ConnectingCardTest {
    @Test
    fun namesTheVehicleWhenKnown() = assertEquals("Connecting to guta-test", connectingTitle("guta-test"))

    @Test
    fun fallsBackWithoutAName() {
        assertEquals("Connecting", connectingTitle(null))
        assertEquals("Connecting", connectingTitle(""))
    }

    @Test
    fun aSilentLinkSaysLoadingStoppedInsteadOfPromisingProgress() {
        val lost = loadingWords("guta-test", lost = true)
        assertEquals("Signal lost", lost.title)
        assert(lost.detail.startsWith("Loading stopped."))
        assertEquals("Hide", lost.dismiss)
        assertEquals("Connecting to guta-test", loadingWords("guta-test", lost = false).title)
    }

    @Test
    fun aBuiltInRadioThatWillNotOpenSaysWhyInsteadOfTheHint() {
        val busy = JSONObject("""{"links":[{"name":"UDP Link (AutoConnect)","lastError":""},{"name":"Built-in radio","lastError":"Another app is using the built-in radio. Close it to connect."}]}""")
        val open = JSONObject("""{"links":[{"name":"Built-in radio","lastError":""}]}""")
        assertEquals("Another app is using the built-in radio. Close it to connect.", lookingHint("radiomaster-ax12", builtInRadioProblem(busy)))
        assertEquals(BUILT_IN_LOOKING_HINT, lookingHint("radiomaster-ax12", builtInRadioProblem(open)))
        assertEquals(LOOKING_HINT, lookingHint(null, null))
    }
}
