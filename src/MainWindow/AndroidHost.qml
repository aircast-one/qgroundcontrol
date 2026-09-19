import QtQuick

Item {
    id: mainWindow

    Connections {
        target: mainWindow.Window.window
        function onSceneGraphError(error, message) {
            console.warn("Scene graph error on a window that is going away:", message)
        }
    }
}
