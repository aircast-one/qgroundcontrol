package one.aircast.android.ui

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
}
