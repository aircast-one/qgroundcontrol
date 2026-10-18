package one.aircast.android.ui

import androidx.compose.runtime.Composable
import java.net.URI
import java.net.URLDecoder

internal data class DeviceScreenProps(val href: String, val query: Map<String, String>, val onOpen: (String) -> Unit, val onClose: () -> Unit)

internal data class DeviceScreenEntry(val title: String, val screen: @Composable (DeviceScreenProps) -> Unit)

internal val DEVICE_SCREENS: Map<String, DeviceScreenEntry> = mapOf(
    "/telemetry" to DeviceScreenEntry("Telemetry") { DeviceTelemetryScreen(it) },
    "/connect" to DeviceScreenEntry("Ground station links") { DeviceConnectScreen(it) },
    "/flights" to DeviceScreenEntry("Flight log") { DeviceFlightsScreen(it) },
    "/settings/device" to DeviceScreenEntry("Device") { DeviceSettingsScreen(it) },
    "/settings/engineering" to DeviceScreenEntry("Engineering") { DeviceEngineeringScreen(it) },
    "/settings/video" to DeviceScreenEntry("Video engine") { DeviceVideoScreen(it) },
    "/settings/recordings" to DeviceScreenEntry("Recordings") { DeviceRecordingsScreen() },
    "/network" to DeviceScreenEntry("Connection") { DeviceNetworkScreen(it) },
    "/streaming" to DeviceScreenEntry("Cameras") { DeviceCamerasScreen(it) },
    "/settings/camera" to DeviceScreenEntry("Watching & recording") { DeviceCameraSetupScreen(it) },
    "/setup" to DeviceScreenEntry("Device setup") { DeviceSetupScreen(it) },
)

internal data class DevicePath(val path: String, val query: Map<String, String>)

private fun decoded(text: String): String = URLDecoder.decode(text, "UTF-8")

internal fun devicePath(href: String): DevicePath {
    val uri = runCatching { URI(href) }.getOrNull()
    val query = uri?.rawQuery.orEmpty().split('&').filter { it.isNotEmpty() }.associate { pair ->
        decoded(pair.substringBefore('=')) to decoded(pair.substringAfter('=', ""))
    }
    return DevicePath(uri?.path?.ifEmpty { "/" } ?: href.substringBefore('?'), query)
}
