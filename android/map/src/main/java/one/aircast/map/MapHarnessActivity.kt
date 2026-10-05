package one.aircast.map

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier

class MapHarnessActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            AircastTheme {
                CompositionLocalProvider(LocalFlyMapEdits provides remember { FlyMapEdits() }) {
                    PlanMapScreen(
                        Modifier.fillMaxSize(),
                        onClear = { PlanBridge.clearPlan() },
                    )
                }
            }
        }
    }
}
