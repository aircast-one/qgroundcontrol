package one.aircast.android

import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageInstaller
import android.os.Build
import android.util.Log
import androidx.core.content.IntentCompat
import java.io.File
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.update

internal const val PLAY_STORE = "com.android.vending"

internal fun installedFromPlay(installer: String?): Boolean = installer == PLAY_STORE

internal fun installerOf(context: Context): String? =
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
        context.packageManager.getInstallSourceInfo(context.packageName).installingPackageName
    } else {
        @Suppress("DEPRECATION")
        context.packageManager.getInstallerPackageName(context.packageName)
    }

internal enum class InstallFailure(val text: String) {
    DifferentBuild("This copy of Aircast came from a different build, so Android won't update it. Uninstall Aircast, then install it from the release page."),
    Downgrade("Your installed Aircast is newer than this release."),
    NoSpace("There isn't enough space on the phone to install the update."),
    Refused("Android didn't install the update."),
}

internal data class InstallOutcome(val apk: String, val failure: InstallFailure)

private const val DOWNGRADE_CODE = "VERSION_DOWNGRADE"
private val SIGNATURE_MISMATCH = listOf("signatures do not match", "UPDATE_INCOMPATIBLE")
private val DIFFERENT_BUILD_STATUSES = setOf(PackageInstaller.STATUS_FAILURE_CONFLICT, PackageInstaller.STATUS_FAILURE_INCOMPATIBLE)

internal fun installFailure(status: Int, message: String?): InstallFailure? {
    val said = message.orEmpty()
    return when {
        status == PackageInstaller.STATUS_SUCCESS || status == PackageInstaller.STATUS_FAILURE_ABORTED -> null
        said.contains(DOWNGRADE_CODE, ignoreCase = true) -> InstallFailure.Downgrade
        SIGNATURE_MISMATCH.any { said.contains(it, ignoreCase = true) } -> InstallFailure.DifferentBuild
        status == PackageInstaller.STATUS_FAILURE_STORAGE -> InstallFailure.NoSpace
        status in DIFFERENT_BUILD_STATUSES -> InstallFailure.DifferentBuild
        else -> InstallFailure.Refused
    }
}

object AppInstall {
    private const val TAG = "AppInstall"
    private const val STATUS_ACTION = "one.aircast.android.INSTALL_STATUS"
    private const val APK_EXTRA = "one.aircast.android.INSTALL_APK"

    internal val outcome = MutableStateFlow<InstallOutcome?>(null)

    internal fun forget(keep: (InstallOutcome) -> Boolean) = outcome.update { held -> held?.takeIf(keep) }

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
                session.commit(statusIntent(context, apk).intentSender)
            }
        }.exceptionOrNull()?.let { failed ->
            Log.w(TAG, "could not hand ${apk.name} to the installer", failed)
            outcome.value = InstallOutcome(apk.absolutePath, InstallFailure.Refused)
        }
    }

    private fun statusIntent(context: Context, apk: File): PendingIntent = PendingIntent.getBroadcast(
        context,
        0,
        Intent(context, InstallStatusReceiver::class.java).setAction(STATUS_ACTION).putExtra(APK_EXTRA, apk.absolutePath),
        PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_MUTABLE,
    )

    fun onStatus(context: Context, intent: Intent) {
        val status = intent.getIntExtra(PackageInstaller.EXTRA_STATUS, PackageInstaller.STATUS_FAILURE)
        val message = intent.getStringExtra(PackageInstaller.EXTRA_STATUS_MESSAGE)
        when (status) {
            PackageInstaller.STATUS_PENDING_USER_ACTION ->
                IntentCompat.getParcelableExtra(intent, Intent.EXTRA_INTENT, Intent::class.java)
                    ?.let { context.startActivity(it.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)) }
            else -> {
                Log.i(TAG, "install finished with status $status: $message")
                outcome.value = installFailure(status, message)?.let { InstallOutcome(intent.getStringExtra(APK_EXTRA).orEmpty(), it) }
            }
        }
    }
}

class InstallStatusReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) = AppInstall.onStatus(context, intent)
}
