import SwiftUI

let ADVANCED_UI_PATH = "view.advancedUi"
let SET_ADVANCED_UI = "advancedUi.set"

func advancedUiShown(_ view: JSON?) -> Bool { view?["shown"].bool(true) ?? true }

@propertyWrapper
struct AdvancedUiShown: DynamicProperty {
    @QgcPath(ADVANCED_UI_PATH) private var view

    var wrappedValue: Bool { advancedUiShown(view) }
}

func toggleAdvancedUi(_ shown: Bool) {
    offMainInOrder { Qgc.invoke(SET_ADVANCED_UI, !shown) }
}
