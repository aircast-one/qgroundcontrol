package org.qtproject.qt.android

import android.app.Activity

object QtRelaunchGuard {
    fun forgetActivity(activity: Activity) {
        val delegate = QtEmbeddedViewInterfaceFactory.create(activity)
        if (delegate is QtNative.AppStateDetailsListener) QtNative.unregisterAppStateListener(delegate)
        QtEmbeddedViewInterfaceFactory.remove(activity)
    }
}
