package one.aircast.android.ui

import android.view.SurfaceHolder
import android.view.SurfaceView
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.viewinterop.AndroidView
import org.mavlink.qgroundcontrol.QGCBridge

internal fun photorealChosen(view: org.json.JSONObject?): Boolean = view?.optBoolean("photoreal") == true

private val surfaceHandoff = object : SurfaceHolder.Callback {
    override fun surfaceCreated(holder: SurfaceHolder) = Unit

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        QGCBridge.photorealSetSurface(holder.surface)
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        QGCBridge.photorealSetSurface(null)
    }
}

@Composable
internal fun PhotorealSurface(modifier: Modifier = Modifier) {
    AndroidView({ context -> SurfaceView(context).apply { holder.addCallback(surfaceHandoff) } }, modifier)
}
