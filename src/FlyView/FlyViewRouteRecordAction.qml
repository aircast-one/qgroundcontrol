import QGroundControl
import QGroundControl.Controls

ToolStripAction {
    text:       _recorder.recording ? qsTr("Stop") : qsTr("Route")
    iconSource: _recorder.recording ? "/res/Stop.svg" : "/res/waypoint.svg"
    visible:    _activeVehicle || _recorder.recording

    property var _activeVehicle: QGroundControl.multiVehicleManager.activeVehicle
    property var _recorder:      globals.routeRecorder

    onTriggered: _recorder.recording ? _recorder.stop() : _recorder.start(_activeVehicle)
}
