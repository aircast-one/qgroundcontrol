package one.aircast.android

import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageInstaller
import android.os.Build
import androidx.core.content.IntentCompat
import java.io.File
import kotlinx.coroutines.flow.MutableStateFlow

internal const val PLAY_STORE = "com.android.vending"

internal fun installedFromPlay(installer: String?): Boolean = installer == PLAY_STORE

internal fun installerOf(context: Context): String? =
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
        context.packageManager.getInstallSourceInfo(context.packageName).installingPackageName
    } else {
        @Suppress("DEPRECATION")
        context.packageManager.getInstallerPackageName(context.packageName)
    }

internal fun installFailure(status: Int, message: String?): String = when (status) {
    PackageInstaller.STATUS_FAILURE_ABORTED -> "The install was cancelled."
    else -> message?.takeIf { it.isNotBlank() } ?: "The installer refused the update (status $status)."
}

object AppInstall {
    private const val STATUS_ACTION = "one.aircast.android.INSTALL_STATUS"

    val outcome = MutableStateFlow<String?>(null)

    fun start(context: Context, apk: File) {
        outcome.value = null
        val installer = context.packageManager.packageInstaller
        val params = PackageInstaller.SessionParams(PackageInstaller.SessionParams.MODE_FULL_INSTALL).apply {
            setAppPackageName(context.packageName)
            setSize(apk.length())
        }
        runCatching {
            installer.openSession(installer.createSession(params)).use { session ->
                session.openWrite(apk.name, 0, apk.length()).use { out ->
                    apk.inputStream().use { it.copyTo(out) }
                    session.fsync(out)
                }
                session.commit(statusIntent(context).intentSender)
            }
        }.exceptionOrNull()?.let { outcome.value = "Could not hand ${apk.name} to the installer: ${it.message}" }
    }

    private fun statusIntent(context: Context): PendingIntent = PendingIntent.getBroadcast(
        context,
        0,
        Intent(context, InstallStatusReceiver::class.java).setAction(STATUS_ACTION),
        PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_MUTABLE,
    )

    fun onStatus(context: Context, intent: Intent) {
        when (val status = intent.getIntExtra(PackageInstaller.EXTRA_STATUS, PackageInstaller.STATUS_FAILURE)) {
            PackageInstaller.STATUS_PENDING_USER_ACTION ->
                IntentCompat.getParcelableExtra(intent, Intent.EXTRA_INTENT, Intent::class.java)
                    ?.let { context.startActivity(it.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)) }
            PackageInstaller.STATUS_SUCCESS -> outcome.value = null
            else -> outcome.value = installFailure(status, intent.getStringExtra(PackageInstaller.EXTRA_STATUS_MESSAGE))
        }
    }
}

class InstallStatusReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) = AppInstall.onStatus(context, intent)
}
