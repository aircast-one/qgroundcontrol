package one.aircast.android

import android.content.Context
import android.hardware.input.InputManager
import android.view.InputDevice
import android.view.KeyEvent
import android.view.MotionEvent
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import one.aircast.android.bridge.Qgc
import org.json.JSONArray
import org.json.JSONObject
import kotlin.math.roundToInt

internal const val JOYSTICK_DEVICES = "joystick.devices"
internal const val JOYSTICK_INPUT = "joystick.input"
private const val SAMPLE_MS = 20L
private const val AXIS_SCALE = 32767f
private const val HAT_UP = 0x01
private const val HAT_RIGHT = 0x02
private const val HAT_DOWN = 0x04
private const val HAT_LEFT = 0x08
private const val HAT_THRESHOLD = 0.5f

internal val GAMEPAD_BUTTONS = listOf(
    KeyEvent.KEYCODE_BUTTON_A, KeyEvent.KEYCODE_BUTTON_B, KeyEvent.KEYCODE_BUTTON_X, KeyEvent.KEYCODE_BUTTON_Y,
    KeyEvent.KEYCODE_BUTTON_L1, KeyEvent.KEYCODE_BUTTON_R1, KeyEvent.KEYCODE_BUTTON_L2, KeyEvent.KEYCODE_BUTTON_R2,
    KeyEvent.KEYCODE_BUTTON_SELECT, KeyEvent.KEYCODE_BUTTON_START, KeyEvent.KEYCODE_BUTTON_MODE,
    KeyEvent.KEYCODE_BUTTON_THUMBL, KeyEvent.KEYCODE_BUTTON_THUMBR,
    KeyEvent.KEYCODE_BUTTON_1, KeyEvent.KEYCODE_BUTTON_2, KeyEvent.KEYCODE_BUTTON_3, KeyEvent.KEYCODE_BUTTON_4,
    KeyEvent.KEYCODE_BUTTON_5, KeyEvent.KEYCODE_BUTTON_6, KeyEvent.KEYCODE_BUTTON_7, KeyEvent.KEYCODE_BUTTON_8,
)

private val HAT_AXES = setOf(MotionEvent.AXIS_HAT_X, MotionEvent.AXIS_HAT_Y)

internal fun hatBits(x: Float, y: Float): Int =
    (if (y < -HAT_THRESHOLD) HAT_UP else 0) or (if (y > HAT_THRESHOLD) HAT_DOWN else 0) or
        (if (x < -HAT_THRESHOLD) HAT_LEFT else 0) or (if (x > HAT_THRESHOLD) HAT_RIGHT else 0)

internal fun scaled(value: Float): Int = (value.coerceIn(-1f, 1f) * AXIS_SCALE).roundToInt()

private fun isGamepad(device: InputDevice): Boolean =
    !device.isVirtual && (device.sources and InputDevice.SOURCE_JOYSTICK == InputDevice.SOURCE_JOYSTICK || device.sources and InputDevice.SOURCE_GAMEPAD == InputDevice.SOURCE_GAMEPAD)

private class Pad(val device: InputDevice) {
    val axes: List<Int> = device.motionRanges.filter { it.source and InputDevice.SOURCE_JOYSTICK != 0 }.map { it.axis }.filter { it !in HAT_AXES }.distinct()
    val hasHat = device.motionRanges.any { it.axis in HAT_AXES }
    val values = FloatArray(axes.size)
    val pressed = BooleanArray(GAMEPAD_BUTTONS.size)
    var hat = 0
}

object GamepadInput : InputManager.InputDeviceListener {
    private val pads = mutableMapOf<Int, Pad>()
    private var sampler: Job? = null

    fun start(context: Context, scope: CoroutineScope) {
        val manager = context.getSystemService(Context.INPUT_SERVICE) as InputManager
        manager.registerInputDeviceListener(this, null)
        rescan()
        sampler?.cancel()
        sampler = scope.launch(Dispatchers.Default) {
            while (isActive) {
                synchronized(pads) { pads.values.map { pad -> Triple(pad.device.name, pad.values.map(::scaled), pad.pressed.toList() to pad.hat) } }
                    .forEach { (name, axes, rest) ->
                        Qgc.invoke(JOYSTICK_INPUT, name, JSONArray(axes), JSONArray(rest.first), JSONArray(listOf(rest.second)))
                    }
                delay(SAMPLE_MS)
            }
        }
    }

    private fun rescan() {
        val found: List<InputDevice> = InputDevice.getDeviceIds().toList().mapNotNull { id -> InputDevice.getDevice(id) }.filter { device -> isGamepad(device) }
        val present: Set<Int> = found.map { device -> device.id }.toSet()
        synchronized(pads) {
            pads.keys.retainAll(present)
            found.forEach { device -> pads.getOrPut(device.id) { Pad(device) } }
        }
        val devices = JSONArray(
            synchronized(pads) {
                pads.values.map { pad -> JSONObject().put("name", pad.device.name).put("axes", pad.axes.size).put("buttons", GAMEPAD_BUTTONS.size).put("hats", if (pad.hasHat) 1 else 0) }
            },
        )
        Qgc.invoke(JOYSTICK_DEVICES, devices)
    }

    fun onMotion(event: MotionEvent): Boolean {
        val pad = synchronized(pads) { pads[event.deviceId] } ?: return false
        if (event.source and InputDevice.SOURCE_JOYSTICK == 0) return false
        synchronized(pads) {
            pad.axes.forEachIndexed { index, axis -> pad.values[index] = event.getAxisValue(axis) }
            if (pad.hasHat) pad.hat = hatBits(event.getAxisValue(MotionEvent.AXIS_HAT_X), event.getAxisValue(MotionEvent.AXIS_HAT_Y))
        }
        return true
    }

    fun onKey(event: KeyEvent): Boolean {
        val pad = synchronized(pads) { pads[event.deviceId] } ?: return false
        val index = GAMEPAD_BUTTONS.indexOf(event.keyCode).takeIf { it >= 0 } ?: return false
        synchronized(pads) { pad.pressed[index] = event.action == KeyEvent.ACTION_DOWN }
        return true
    }

    override fun onInputDeviceAdded(deviceId: Int) = rescan()

    override fun onInputDeviceRemoved(deviceId: Int) = rescan()

    override fun onInputDeviceChanged(deviceId: Int) = rescan()
}
