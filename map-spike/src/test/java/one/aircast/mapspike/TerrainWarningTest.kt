package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class TerrainWarningTest {
    private fun clearance(
        collides: Boolean = false,
        metres: Double? = null,
        text: String = "",
        complete: Boolean = true,
    ) = Clearance(collides, metres, text, complete)

    @Test
    fun `a route below the ground is named with how far below`() {
        assertEquals(
            "The route goes 150.0 m below the ground.",
            terrainWarning(clearance(collides = true, metres = -150.0, text = "150.0 m")),
        )
    }

    @Test
    fun `a route below the ground is still named when the ground is only partly known`() {
        assertEquals(
            "The route goes 12.0 m below the ground.",
            terrainWarning(
                clearance(collides = true, metres = -12.0, text = "12.0 m", complete = false),
            ),
        )
    }

    @Test
    fun `a route that clears is not announced, because clearing is the ordinary case`() {
        assertNull(terrainWarning(clearance(metres = 50.0, text = "50.0 m")))
    }

    @Test
    fun `a clearance measured over ground that is only partly known says nothing`() {
        assertNull(terrainWarning(clearance(metres = 4.0, text = "4.0 m", complete = false)))
    }

    @Test
    fun `no terrain at all says nothing rather than guessing either way`() {
        assertNull(terrainWarning(clearance(metres = null, complete = false)))
        assertNull(terrainWarning(null))
    }

    @Test
    fun `a collision with no figure still warns rather than falling silent`() {
        assertEquals(
            "The route goes below the ground.",
            terrainWarning(clearance(collides = true, metres = null, text = "")),
        )
    }

    @Test
    fun `the clearance is read from the view the core serves`() {
        val view = JSONObject(
            """{"points":[],"hasCollision":true,"minClearanceMetres":-8.5,""" +
                """"clearanceText":"8.5 m","clearanceComplete":true}""",
        )
        val read = clearanceOf(view)

        assertEquals(true, read?.collides)
        assertEquals(-8.5, read?.metres ?: 0.0, 1e-9)
        assertEquals("8.5 m", read?.text)
    }

    @Test
    fun `a null clearance from the core reads as unknown, not as zero`() {
        val view = JSONObject(
            """{"points":[],"hasCollision":false,"minClearanceMetres":null,""" +
                """"clearanceText":null,"clearanceComplete":false}""",
        )

        assertNull(clearanceOf(view)?.metres)
        assertEquals("", clearanceOf(view)?.text)
    }
}
