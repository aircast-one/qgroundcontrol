package one.aircast.android

import android.content.pm.ActivityInfo
import one.aircast.android.ui.FlyView
import org.junit.Assert.assertEquals
import org.junit.Test

class DebugUiTest {
    @Test
    fun `each command parses to what the app applies`() {
        assertEquals(DebugCommand.ShowTab(Tab.Plan), debugCommand("tab", "plan").getOrThrow())
        assertEquals(DebugCommand.ShowFlyView(FlyView.Video), debugCommand("fly-view", "Video").getOrThrow())
        assertEquals(DebugCommand.Orient(ActivityInfo.SCREEN_ORIENTATION_LANDSCAPE), debugCommand("orientation", "landscape").getOrThrow())
        assertEquals(DebugCommand.EditLayout(false), debugCommand("layout-edit", "off").getOrThrow())
        assertEquals(DebugCommand.ResetLayout, debugCommand("layout-reset", null).getOrThrow())
        assertEquals(DebugCommand.Open("takeoff"), debugCommand("open", "takeoff").getOrThrow())
    }

    @Test
    fun `a bad command names what it accepts instead of guessing`() {
        assertEquals("cmd must be one of $DEBUG_COMMANDS", debugCommand("rotate", null).exceptionOrNull()?.message)
        assertEquals("orientation must be one of landscape, reverse-landscape, portrait, auto", debugCommand("orientation", "sideways").exceptionOrNull()?.message)
        assertEquals("tab must be one of Fly, Plan, Setup, Analyze, Settings", debugCommand("tab", "Home").exceptionOrNull()?.message)
        assertEquals(
            "open needs a sheet (more, readings, camera, gimbal, indicators, status, modes) or a flight action id, e.g. takeoff, rtl, land",
            debugCommand("open", "").exceptionOrNull()?.message,
        )
    }
}
