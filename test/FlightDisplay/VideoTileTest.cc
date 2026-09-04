#include "VideoTileTest.h"
#include "QuickInteractionTestHelpers.h"

#include <QtQml/QQmlContext>
#include <QtQml/QQmlPropertyMap>

#include "SettingsManager.h"
#include "VideoSettings.h"

static bool loadVideoView(QQuickView& view, QQmlPropertyMap& globals)
{
    globals.insert(QStringLiteral("activeVehicle"), QVariant());
    view.engine()->rootContext()->setContextProperty(QStringLiteral("globals"), &globals);
    return loadTestView(view, QStringLiteral("qrc:/unittest/VideoTileTest.qml"));
}

static QQuickItem* findItem(QQuickItem* item, const std::function<bool(QQuickItem*)>& match)
{
    if (match(item)) {
        return item;
    }
    const QList<QQuickItem*> children = item->childItems();
    for (QQuickItem* child : children) {
        if (QQuickItem* found = findItem(child, match)) {
            return found;
        }
    }
    return nullptr;
}

static QQuickItem* findNamed(QQuickView& view, const QString& name)
{
    return findItem(view.rootObject(), [&name](QQuickItem* item) { return item->objectName() == name; });
}

static QQuickItem* findTile(QQuickView& view, int cameraNumber)
{
    return findItem(view.rootObject(), [cameraNumber](QQuickItem* item) {
        return item->objectName().startsWith(QLatin1String("videoTile")) && item->property("cameraNumber").toInt() == cameraNumber;
    });
}

void VideoTileTest::_tuckPersistsAcrossReload()
{
    clearQmlGlobalSettings({"VideoRailTucked"});

    {
        QQmlPropertyMap globals;
        QQuickView view;
        QVERIFY(loadVideoView(view, globals));

        QQuickItem* layer = findNamed(view, QStringLiteral("tiles"));
        QVERIFY(layer);
        QVERIFY(!layer->property("tucked").toBool());

        QVERIFY(QMetaObject::invokeMethod(layer, "setTucked", Q_ARG(QVariant, QVariant(true))));
        QVERIFY(layer->property("tucked").toBool());
    }

    {
        QQmlPropertyMap globals2;
        QQuickView view2;
        QVERIFY(loadVideoView(view2, globals2));

        QQuickItem* layer2 = findNamed(view2, QStringLiteral("tiles"));
        QVERIFY(layer2);
        QVERIFY(layer2->property("tucked").toBool());

        QVERIFY(QMetaObject::invokeMethod(layer2, "setTucked", Q_ARG(QVariant, QVariant(false))));
        QVERIFY(!layer2->property("tucked").toBool());
    }

    QQmlPropertyMap globals3;
    QQuickView view3;
    QVERIFY(loadVideoView(view3, globals3));

    QQuickItem* layer3 = findNamed(view3, QStringLiteral("tiles"));
    QVERIFY(layer3);
    QVERIFY(!layer3->property("tucked").toBool());
}

void VideoTileTest::_extraCameraTileAttachedToPip()
{
    VideoSettings* const videoSettings = SettingsManager::instance()->videoSettings();
    const QVariant savedSources = videoSettings->extraVideoSources()->rawValue();
    const QVariant savedMultiView = videoSettings->multiViewEnabled()->rawValue();
    const QVariant savedVideoSource = videoSettings->videoSource()->rawValue();

    videoSettings->videoSource()->setRawValue(videoSettings->rtspVideoSource());
    videoSettings->extraVideoSources()->setRawValue(
        QStringLiteral(R"([{"name":"Cam2","source":"RTSP Video Stream","url":"rtsp://127.0.0.1/2"}])"));
    videoSettings->multiViewEnabled()->setRawValue(true);

    QQmlPropertyMap globals;
    QQuickView view;
    QVERIFY(loadVideoView(view, globals));

    QQuickItem* tile = findTile(view, 2);
    QVERIFY(tile);
    QCOMPARE(tile->property("cameraNumber").toInt(), 2);
    QVERIFY(tile->isVisible());

    QQuickItem* pip = findNamed(view, QStringLiteral("pip"));
    QQuickItem* rail = findNamed(view, QStringLiteral("videoRail"));
    QVERIFY(pip);
    QVERIFY(rail);
    QVERIFY(rail->isVisible());

    const QRectF pipRect(pip->mapToScene(QPointF(0, 0)), pip->size());
    const QRectF railRect(rail->mapToScene(QPointF(0, 0)), rail->size());
    QVERIFY(railRect.left() > pipRect.right());
    QCOMPARE(railRect.bottom(), pipRect.bottom());

    videoSettings->videoSource()->setRawValue(savedVideoSource);
    videoSettings->extraVideoSources()->setRawValue(savedSources);
    videoSettings->multiViewEnabled()->setRawValue(savedMultiView);
}

void VideoTileTest::_gridPersistsAcrossReload()
{
    clearQmlGlobalSettings({"VideoRailGrid"});

    {
        QQmlPropertyMap globals;
        QQuickView view;
        QVERIFY(loadVideoView(view, globals));

        QQuickItem* layer = findNamed(view, QStringLiteral("tiles"));
        QVERIFY(layer);
        QVERIFY(!layer->property("grid").toBool());
        QVERIFY(QMetaObject::invokeMethod(layer, "setGrid", Q_ARG(QVariant, QVariant(true))));
        QVERIFY(layer->property("grid").toBool());
    }

    QQmlPropertyMap globals2;
    QQuickView view2;
    QVERIFY(loadVideoView(view2, globals2));

    QQuickItem* layer2 = findNamed(view2, QStringLiteral("tiles"));
    QVERIFY(layer2);
    QVERIFY(layer2->property("grid").toBool());
    QVERIFY(QMetaObject::invokeMethod(layer2, "setGrid", Q_ARG(QVariant, QVariant(false))));
}

void VideoTileTest::_focusLayoutOverflowsIntoMore()
{
    clearQmlGlobalSettings({"VideoRailGrid"});

    VideoSettings* const videoSettings = SettingsManager::instance()->videoSettings();
    const QVariant savedSources = videoSettings->extraVideoSources()->rawValue();
    const QVariant savedMultiView = videoSettings->multiViewEnabled()->rawValue();
    const QVariant savedVideoSource = videoSettings->videoSource()->rawValue();

    videoSettings->videoSource()->setRawValue(videoSettings->rtspVideoSource());
    videoSettings->extraVideoSources()->setRawValue(QStringLiteral(R"([
        {"name":"Cam2","source":"RTSP Video Stream","url":"rtsp://127.0.0.1/2"},
        {"name":"Cam3","source":"RTSP Video Stream","url":"rtsp://127.0.0.1/3"},
        {"name":"Cam4","source":"RTSP Video Stream","url":"rtsp://127.0.0.1/4"},
        {"name":"Cam5","source":"RTSP Video Stream","url":"rtsp://127.0.0.1/5"}])"));
    videoSettings->multiViewEnabled()->setRawValue(true);

    QQmlPropertyMap globals;
    QQuickView view;
    QVERIFY(loadVideoView(view, globals));

    QQuickItem* layer = findNamed(view, QStringLiteral("tiles"));
    QQuickItem* more = findNamed(view, QStringLiteral("videoTileMore"));
    QVERIFY(layer);
    QVERIFY(more);
    QVERIFY(findTile(view, 4)->isVisible());
    QVERIFY(!findTile(view, 5)->isVisible());
    QVERIFY(more->isVisible());

    QVERIFY(QMetaObject::invokeMethod(layer, "setGrid", Q_ARG(QVariant, QVariant(true))));
    QVERIFY(findTile(view, 5)->isVisible());
    QVERIFY(!more->isVisible());

    QQuickItem* pip = findNamed(view, QStringLiteral("pip"));
    QQuickItem* tile = findTile(view, 2);
    QTRY_COMPARE(tile->width(), pip->width());
    QVERIFY(tile->x() > 0 || tile->y() > 0);

    QVERIFY(QMetaObject::invokeMethod(layer, "setGrid", Q_ARG(QVariant, QVariant(false))));
    videoSettings->videoSource()->setRawValue(savedVideoSource);
    videoSettings->extraVideoSources()->setRawValue(savedSources);
    videoSettings->multiViewEnabled()->setRawValue(savedMultiView);
}

// The "no video" pill carries Retry and Video Settings. Nothing in the rig knows about it
// unless FlyViewVideo says so, and a registration that silently never happens looks exactly
// like one that works - the chips simply sit on top of the buttons.
void VideoTileTest::_statusPillRegistersAsAnObstacleOwnedByThePip()
{
    QQuickView view;
    QQmlPropertyMap globals;
    globals.insert(QStringLiteral("activeVehicle"), QVariant());
    view.engine()->rootContext()->setContextProperty(QStringLiteral("globals"), &globals);
    QVERIFY(loadTestView(view, QStringLiteral("qrc:/unittest/StatusPillTest.qml")));

    QQuickItem *const pill = findNamed(view, QStringLiteral("videoStatusPill"));
    QQuickItem *const pip  = findNamed(view, QStringLiteral("pip"));
    QVERIFY(pill && pip);

    const QVariantList statics = view.rootObject()->property("registeredStatics").toList();
    const QVariantList owners  = view.rootObject()->property("registeredOwners").toList();
    QCOMPARE(statics.count(), 1);
    QCOMPARE(statics.first().value<QQuickItem*>(), pill);

    // Owned by the pip because the pill lives inside the video, and the video is sometimes the
    // pip itself. Without the owner the rig pushes the pip away from its own contents.
    QCOMPARE(owners.count(), 1);
    QCOMPARE(owners.first().value<QQuickItem*>(), pip);
}
