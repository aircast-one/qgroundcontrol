package one.aircast.android.ui

import androidx.compose.ui.geometry.Offset
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class GeometryImageTest {
    private val quad = listOf(
        GeometryMotor(0, 1, 1.0, 1.0, true),
        GeometryMotor(1, 2, -1.0, -1.0, true),
        GeometryMotor(2, 3, 1.0, -1.0, false),
        GeometryMotor(3, 4, -1.0, 1.0, false),
    )

    @Test
    fun `the quad is laid out nose up, centred and inside the image`() {
        val layout = geometryLayout(quad, 320f, 160f)!!
        val front = layout.motors.first { it.motor.label == 1 }
        val back = layout.motors.first { it.motor.label == 2 }
        assertTrue("front is above back, as +x is drawn upwards", front.center.y < back.center.y)
        assertTrue("+y is drawn to the right", front.center.x > back.center.x)
        assertEquals(layout.origin.x, (front.center.x + back.center.x) / 2f, 0.01f)
        assertTrue(layout.motors.all { it.center.x - layout.rotorDiameter / 2f >= 0f && it.center.y + layout.rotorDiameter / 2f <= 160f })
        assertNull("one motor draws nothing", geometryLayout(quad.take(1), 320f, 160f))
    }

    @Test
    fun `a coaxial pair draws the lower motor offset and a tap picks only a highlighted motor`() {
        val coax = quad + GeometryMotor(4, 5, 1.0, 1.0, false)
        val layout = geometryLayout(coax, 320f, 160f)!!
        val lower = layout.motors.first { it.motor.label == 5 }
        assertTrue(lower.coax)
        assertTrue(layout.extraYMargin > 0f)
        val target = layout.motors.first { it.motor.label == 3 }
        assertEquals(2, motorAt(layout, target.textCenter, setOf(2)))
        assertNull(motorAt(layout, target.textCenter, emptySet()))
        assertNull(motorAt(layout, Offset(-50f, -50f), setOf(0, 1, 2, 3)))
    }
}
