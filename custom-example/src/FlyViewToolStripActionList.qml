import QtQml.Models

import QGroundControl
import QGroundControl.Controls
import QGroundControl.FlyView
import QGroundControl.Viewer3D

ToolStripActionList {
    id: _root

    signal displayPreFlightChecklist

    model: [
        Viewer3DShowAction { },
        PreFlightCheckListShowAction { onTriggered: displayPreFlightChecklist() },
        GuidedActionLand { },
        GuidedActionRTL { },
        GuidedActionPause { },
        FlyViewAdditionalActionsButton { },
        GuidedToolStripAction {
            text:       _guidedController._customController.customButtonTitle
            iconSource: "/res/gear-white.svg"
            visible:    true
            enabled:    true
            actionID:   _guidedController._customController.actionCustomButton
        }
    ]
}
