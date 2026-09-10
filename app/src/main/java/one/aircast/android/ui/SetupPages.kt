package one.aircast.android.ui

internal const val SENSORS = "Sensors"
internal const val RADIO = "Radio"
internal const val REMOTE_SUPPORT = "Remote Support"

internal fun headCanOpen(page: SetupPage?, name: String): Boolean =
    page != null &&
        page.native &&
        (name == SENSORS || name == RADIO || name == REMOTE_SUPPORT || page.parameterSections)
