package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class LogReplayNameTest {
    @Test
    fun theReplayLinkIsNamedAfterThePickedFileLikeLinkManager() {
        assertEquals("2026-10-03 10-11-12.tlog", replayFileName("2026-10-03 10-11-12.tlog"))
        assertEquals("flight.tlog", replayFileName("../../flight.tlog"))
        assertEquals("log-replay.tlog", replayFileName(null))
        assertEquals("log-replay.tlog", replayFileName(".."))
    }
}
