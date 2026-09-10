import QtQuick

// Qt 6.8.3 has no public way to start Qt embedded without a QtQuickView: QtView,
// QtEmbeddedLoader and QtEmbeddedDelegate are all package private, and QtActivityBase
// is the model where Qt owns the Activity. So this file is not a UI - it is the
// smallest thing that view can load, and the head renders everything itself.
Item {
    id: mainWindow
}
