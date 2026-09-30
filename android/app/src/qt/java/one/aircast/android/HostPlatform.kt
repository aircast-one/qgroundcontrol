package one.aircast.android

import android.app.Activity
import android.view.View
import org.mavlink.qgroundcontrol.QGCSDLManager
import org.mavlink.qgroundcontrol.QGCUsbSerialManager
import org.qtproject.qt.android.QtQuickView
import org.qtproject.qt.android.QtRelaunchGuard

private const val QML_URI = "qrc:/qml/QGroundControl/MainWindow/AndroidHost.qml"
private const val QML_LIBRARY = "AircastQGC"

object HostPlatform {
    fun start(activity: Activity): View? {
        QGCUsbSerialManager.initialize(activity)
        QGCSDLManager.initialize(activity)
        return QtQuickView(activity, QML_URI, QML_LIBRARY, arrayOf("qrc:/qml"))
    }

    fun stop(activity: Activity) {
        if (activity.isChangingConfigurations) QtRelaunchGuard.forgetActivity(activity)
        runCatching { QGCSDLManager.cleanup() }
        runCatching { QGCUsbSerialManager.cleanup(activity) }
    }
}
