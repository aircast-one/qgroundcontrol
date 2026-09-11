package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class HandleHitTest {
    @Test
    fun `each handle kind gives its own hit`() {
        assertEquals(MapHit.FenceVertex(1, 2), handleHit(HANDLE_KIND_FENCE, 1, 2))
        assertEquals(MapHit.SurveyVertex(1, 2), handleHit(HANDLE_KIND_SURVEY, 1, 2))
        assertEquals(MapHit.CircleCentre(1), handleHit(HANDLE_KIND_CIRCLE, 1, 2))
    }

    @Test
    fun `a handle kind nobody knows is not a fence vertex`() {
        assertNull(handleHit("corridor", 1, 2))
        assertNull(handleHit(null, 1, 2))
        assertNull(handleHit("", 1, 2))
    }
}

class WithinTapTest {
    @Test
    fun `a finger that barely moves is a tap`() {
        assertTrue(withinTap(0f, 0f))
        assertTrue(withinTap(TAP_SLOP_PX, -TAP_SLOP_PX))
    }

    @Test
    fun `a finger that travels is a pan, and must not clear the selection`() {
        assertFalse(withinTap(TAP_SLOP_PX + 1f, 0f))
        assertFalse(withinTap(0f, TAP_SLOP_PX + 1f))
    }
}
