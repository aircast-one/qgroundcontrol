package one.aircast.map

import android.content.Context
import androidx.compose.runtime.mutableStateOf

object Map3d {
    private const val STORE = "fly-map-3d"
    private const val SHOWN = "shown"
    private val shown = mutableStateOf(false)

    val isShown: Boolean get() = shown.value

    fun load(context: Context) {
        shown.value = store(context).getBoolean(SHOWN, false)
    }

    fun show(context: Context, on: Boolean) {
        shown.value = on
        store(context).edit().putBoolean(SHOWN, on).apply()
    }

    private fun store(context: Context) = context.getSharedPreferences(STORE, Context.MODE_PRIVATE)
}
