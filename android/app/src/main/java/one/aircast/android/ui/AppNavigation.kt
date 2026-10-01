package one.aircast.android.ui

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

internal object AppNavigation {
    var settingsPage by mutableStateOf<String?>(null)
}
