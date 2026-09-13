package one.aircast.android.ui

import one.aircast.android.bridge.Fact


data class SettingsSection(val title: String, val factNames: List<String>)

internal const val OTHER_SECTION = "Other"

internal val INTERNAL_FACTS = setOf(
    "firstRunPromptIdsShown",
    "instrumentQmlFile2",
)

internal val FACTS_WITH_EDITORS = setOf(
    "rcControls",
    "extraVideoSources",
)

internal fun hiddenFacts(): Set<String> = INTERNAL_FACTS + FACTS_WITH_EDITORS

private val APP_SECTIONS = listOf(
    SettingsSection(
        "Appearance",
        listOf("indoorPalette", "appFontPointSize", "overlayGlassFrost", "qLocaleLanguage"),
    ),
    SettingsSection("Sound", listOf("audioMuted", "batteryPercentRemainingAnnounce")),
    SettingsSection("Preflight checklist", listOf("useChecklist", "enforceChecklist")),
    SettingsSection(
        "Virtual joystick",
        listOf("virtualJoystick", "virtualJoystickAutoCenterThrottle", "virtualJoystickLeftHandedMode"),
    ),
    SettingsSection(
        "Planning defaults",
        listOf(
            "defaultMissionItemAltitude", "offlineEditingFirmwareClass", "offlineEditingVehicleClass",
            "offlineEditingCruiseSpeed", "offlineEditingHoverSpeed", "offlineEditingAscentSpeed",
            "offlineEditingDescentSpeed",
        ),
    ),
    SettingsSection("Map providers", listOf("mapboxToken", "mapboxAccount", "mapboxStyle", "esriToken", "vworldToken", "customURL")),
    SettingsSection("AirLink", listOf("loginAirLink", "passAirLink")),
    SettingsSection("Files", listOf("savePath", "androidSaveToSDCard", "disableAllPersistence")),
)

private val VIDEO_SECTIONS = listOf(
    SettingsSection("Cameras", listOf("videoSource", "primaryCameraName", "activeVideoSource", "multiViewEnabled")),
    SettingsSection(
        "Stream",
        listOf("udpUrl", "rtspUrl", "tcpUrl", "whepUrl", "rtspTimeout", "streamEnabled",
               "disableWhenDisarmed", "lowLatencyMode", "forceVideoDecoder"),
    ),
    SettingsSection("Display", listOf("videoFit", "aspectRatio", "gridLines", "showRecControl")),
    SettingsSection(
        "Recording",
        listOf("videoSavePath", "recordingFormat", "enableStorageLimit", "maxVideoSize"),
    ),
)

private val FLY_VIEW_SECTIONS = listOf(
    SettingsSection(
        "Guided commands",
        listOf("guidedMinimumAltitude", "guidedMaximumAltitude", "maxGoToLocationDistance",
               "forwardFlightGoToLocationLoiterRad", "goToLocationRequiresConfirmInGuided",
               "updateHomePosition"),
    ),
    SettingsSection(
        "Map and compass",
        listOf("keepMapCenteredOnVehicle", "showAdditionalIndicatorsCompass", "lockNoseUpCompass",
               "showObstacleDistanceOverlay"),
    ),
    SettingsSection(
        "On-screen controls",
        listOf("showPhotoVideoControl", "showSimpleCameraControl", "showLogReplayStatusBar"),
    ),
    SettingsSection(
        "Camera and gimbal channels",
        listOf("gimbalTiltChannel", "gimbalPanChannel", "cameraZoomChannel", "cameraLightChannel",
               "cameraRecordChannel"),
    ),
    SettingsSection("Sharing control", listOf("requestControlAllowTakeover", "requestControlTimeout")),
)

internal val SETTINGS_SECTIONS: Map<String, List<SettingsSection>> = mapOf(
    "settings.appSettings" to APP_SECTIONS,
    "settings.videoSettings" to VIDEO_SECTIONS,
    "settings.flyViewSettings" to FLY_VIEW_SECTIONS,
)

internal fun sectionedFacts(page: String, facts: List<Fact>): List<Pair<String, List<Fact>>> {
    val shown = facts.filterNot { it.name in hiddenFacts() }
    val sections = SETTINGS_SECTIONS[page] ?: return listOf("" to shown)

    val byName = shown.associateBy { it.name }
    val named = sections.mapNotNull { section ->
        val members = section.factNames.mapNotNull { byName[it] }
        if (members.isEmpty()) null else section.title to members
    }
    val claimed = sections.flatMap { it.factNames }.toSet()
    val leftovers = shown.filterNot { it.name in claimed }
    return if (leftovers.isEmpty()) named else named + (OTHER_SECTION to leftovers)
}
