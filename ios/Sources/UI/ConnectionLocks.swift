import SwiftUI
import UIKit

func vehicleConnected(_ view: JSON?) -> Bool { (view?["count"].int ?? 0) > 0 }

struct ConnectionLocks: View {
    @QgcPath(VEHICLES_VIEW) private var vehicles

    var body: some View {
        Color.clear
            .frame(width: 0, height: 0)
            .accessibilityHidden(true)
            .onChange(of: vehicleConnected(vehicles), initial: true) { _, connected in
                UIApplication.shared.isIdleTimerDisabled = connected
            }
            .onDisappear { UIApplication.shared.isIdleTimerDisabled = false }
    }
}
