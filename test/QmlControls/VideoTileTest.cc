#include "VideoTileTest.h"
#include "QuickInteractionTestHelpers.h"

#include <QtQml/QQmlContext>
#include <QtQml/QQmlPropertyMap>

// Loads the real FlyViewVideo (tiles included) with a stubbed `globals` context so the
// per-tile collapse state can be exercised end to end against the real persistence path.
static bool loadVideoView(QQuickView& view, QQmlPropertyMap& globals)
{
    globals.insert(QStringLiteral("activeVehicle"), QVariant());
    view.engine()->rootContext()->setContextProperty(QStringLiteral("globals"), &globals);
    return loadTestView(view, QStringLiteral("qrc:/unittest/VideoTileTest.qml"));
}

// Repeater delegates are only reachable through the visual tree, not QObject::findChildren.
static QQuickItem* findTileIn(QQuickItem* item, const QString& key)
{
    if (item->property("_tileExpandedSettingsKey").toString() == key) {
        return item;
    }
    const QList<QQuickItem*> children = item->childItems();
    for (QQuickItem* child : children) {
        if (QQuickItem* found = findTileIn(child, key)) {
            return found;
        }
    }
    return nullptr;
}

// Tiles are keyed by camera number (not slot); with no extra cameras configured every
// slot maps to camera 0, so the first matching tile is the one under test.
static QQuickItem* findTile(QQuickView& view, int cameraNumber)
{
    return findTileIn(view.rootObject(), QStringLiteral("VideoTileCamera%1Expanded").arg(cameraNumber));
}

void VideoTileTest::_collapsePersistsAcrossReload()
{
    clearQmlGlobalSettings({"VideoTileCamera0Expanded"});

    {
        QQmlPropertyMap globals;
        QQuickView view;
        QVERIFY(loadVideoView(view, globals));

        QQuickItem* tile = findTile(view, 0);
        QVERIFY(tile);
        QVERIFY(tile->property("tileExpanded").toBool());

        QVERIFY(QMetaObject::invokeMethod(tile, "setTileExpanded", Q_ARG(QVariant, QVariant(false))));
        QVERIFY(!tile->property("tileExpanded").toBool());
    }

    // The collapsed state must survive a reload.
    {
        QQmlPropertyMap globals2;
        QQuickView view2;
        QVERIFY(loadVideoView(view2, globals2));

        QQuickItem* tile2 = findTile(view2, 0);
        QVERIFY(tile2);
        QVERIFY(!tile2->property("tileExpanded").toBool());

        QVERIFY(QMetaObject::invokeMethod(tile2, "setTileExpanded", Q_ARG(QVariant, QVariant(true))));
        QVERIFY(tile2->property("tileExpanded").toBool());
    }

    // And so must re-expanding.
    QQmlPropertyMap globals3;
    QQuickView view3;
    QVERIFY(loadVideoView(view3, globals3));

    QQuickItem* tile3 = findTile(view3, 0);
    QVERIFY(tile3);
    QVERIFY(tile3->property("tileExpanded").toBool());
}
