#include "VideoTileTest.h"
#include "QuickInteractionTestHelpers.h"

#include <QtQml/QQmlContext>
#include <QtQml/QQmlPropertyMap>

#include <QtCore/QRegularExpression>
#include <QtCore/QScopeGuard>
#include <memory>

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

static auto restoreCamerasOnExit(VideoSettings* videoSettings)
{
    return qScopeGuard([videoSettings,
                        cameras = videoSettings->cameras()->rawValue(),
                        active = videoSettings->activeVideoSource()->rawValue(),
                        multiView = videoSettings->multiViewEnabled()->rawValue()] {
        videoSettings->cameras()->setRawValue(cameras);
        videoSettings->activeVideoSource()->setRawValue(active);
        videoSettings->multiViewEnabled()->setRawValue(multiView);
    });
}

void VideoTileTest::init()
{
    UnitTest::init();
    ignoreLogMessage("qt.qpa.fonts", QtWarningMsg, QRegularExpression(QStringLiteral("Populating font family aliases")));
    ignoreLogMessage("default", QtWarningMsg, QRegularExpression(QStringLiteral("onFlyViewActiveChanged")));
    ignoreLogMessage("default", QtWarningMsg, QRegularExpression(QStringLiteral("mainWindow is not defined")));
    ignoreLogMessage("default", QtWarningMsg, QRegularExpression(QStringLiteral("activeVehicle' of null")));
}

void VideoTileTest::_tuckPersistsAcrossReload()
{
    clearQmlGlobalSettings({"VideoRailTucked"});

    {
        const std::unique_ptr<QQmlPropertyMap> globals(QQmlPropertyMap::create());
        QQuickView view;
        QVERIFY(loadVideoView(view, *globals));

        QQuickItem* layer = findNamed(view, QStringLiteral("tiles"));
        QVERIFY(layer);
        QVERIFY(!layer->property("tucked").toBool());

        QVERIFY(QMetaObject::invokeMethod(layer, "setTucked", Q_ARG(QVariant, QVariant(true))));
        QVERIFY(layer->property("tucked").toBool());
    }

    {
        const std::unique_ptr<QQmlPropertyMap> globals2(QQmlPropertyMap::create());
        QQuickView view2;
        QVERIFY(loadVideoView(view2, *globals2));

        QQuickItem* layer2 = findNamed(view2, QStringLiteral("tiles"));
        QVERIFY(layer2);
        QVERIFY(layer2->property("tucked").toBool());

        QVERIFY(QMetaObject::invokeMethod(layer2, "setTucked", Q_ARG(QVariant, QVariant(false))));
        QVERIFY(!layer2->property("tucked").toBool());
    }

    const std::unique_ptr<QQmlPropertyMap> globals3(QQmlPropertyMap::create());
    QQuickView view3;
    QVERIFY(loadVideoView(view3, *globals3));

    QQuickItem* layer3 = findNamed(view3, QStringLiteral("tiles"));
    QVERIFY(layer3);
    QVERIFY(!layer3->property("tucked").toBool());
}

void VideoTileTest::_extraCameraTileAttachedToPip()
{
    VideoSettings* const videoSettings = SettingsManager::instance()->videoSettings();
    const auto restore = restoreCamerasOnExit(videoSettings);

    videoSettings->cameras()->setRawValue(QStringLiteral(R"([
        {"name":"Cam1","source":"RTSP Video Stream","url":"rtsp://127.0.0.1/1"},
        {"name":"Cam2","source":"RTSP Video Stream","url":"rtsp://127.0.0.1/2"}])"));
    videoSettings->activeVideoSource()->setRawValue(0);
    videoSettings->multiViewEnabled()->setRawValue(true);

    const std::unique_ptr<QQmlPropertyMap> globals(QQmlPropertyMap::create());
    QQuickView view;
    QVERIFY(loadVideoView(view, *globals));

    QQuickItem* tile = findTile(view, 2);
    QVERIFY(tile);
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
}

void VideoTileTest::_gridPersistsAcrossReload()
{
    clearQmlGlobalSettings({"VideoRailGrid"});

    {
        const std::unique_ptr<QQmlPropertyMap> globals(QQmlPropertyMap::create());
        QQuickView view;
        QVERIFY(loadVideoView(view, *globals));

        QQuickItem* layer = findNamed(view, QStringLiteral("tiles"));
        QVERIFY(layer);
        QVERIFY(!layer->property("grid").toBool());
        QVERIFY(QMetaObject::invokeMethod(layer, "setGrid", Q_ARG(QVariant, QVariant(true))));
        QVERIFY(layer->property("grid").toBool());
    }

    const std::unique_ptr<QQmlPropertyMap> globals2(QQmlPropertyMap::create());
    QQuickView view2;
    QVERIFY(loadVideoView(view2, *globals2));

    QQuickItem* layer2 = findNamed(view2, QStringLiteral("tiles"));
    QVERIFY(layer2);
    QVERIFY(layer2->property("grid").toBool());
    QVERIFY(QMetaObject::invokeMethod(layer2, "setGrid", Q_ARG(QVariant, QVariant(false))));
}

void VideoTileTest::_thePipTileFollowsTheActiveCamera()
{
    clearQmlGlobalSettings({"VideoRailGrid"});

    VideoSettings* const videoSettings = SettingsManager::instance()->videoSettings();
    const auto restore = restoreCamerasOnExit(videoSettings);
    videoSettings->cameras()->setRawValue(QStringLiteral(R"([
        {"name":"Belly","source":"UDP h.264 Video Stream","url":"0.0.0.0:5600"},
        {"name":"Radio","source":"UDP h.264 Video Stream","url":"127.0.0.1:5600"},
        {"name":"Cam3","source":"RTSP Video Stream","url":"rtsp://127.0.0.1/3"},
        {"name":"Cam4","source":"RTSP Video Stream","url":"rtsp://127.0.0.1/4"},
        {"name":"Cam5","source":"RTSP Video Stream","url":"rtsp://127.0.0.1/5"}])"));
    videoSettings->multiViewEnabled()->setRawValue(true);
    videoSettings->activeVideoSource()->setRawValue(2);

    const std::unique_ptr<QQmlPropertyMap> globals(QQmlPropertyMap::create());
    QQuickView view;
    QVERIFY(loadVideoView(view, *globals));
    QQuickItem* const layer = findNamed(view, QStringLiteral("tiles"));
    QQuickItem* const tile = findNamed(view, QStringLiteral("videoTile"));
    QQuickItem* const pip = findNamed(view, QStringLiteral("pip"));
    QVERIFY(layer && tile && pip);
    QVERIFY(tile->isVisible());

    const auto pipTileAfterSwitchingTo = [videoSettings, tile](int active) {
        videoSettings->activeVideoSource()->setRawValue(active);
        return tile->property("cameraNumber").toInt();
    };
    QCOMPARE(tile->property("cameraNumber").toInt(), 4);
    QCOMPARE(pipTileAfterSwitchingTo(0), 3);
    QCOMPARE(pipTileAfterSwitchingTo(1), 3);
    QCOMPARE(pipTileAfterSwitchingTo(4), 1);
    QCOMPARE(pipTileAfterSwitchingTo(2), 4);
    QVERIFY(tile->isVisible());

    const auto ungrid = qScopeGuard([layer] { (void) QMetaObject::invokeMethod(layer, "setGrid", Q_ARG(QVariant, QVariant(false))); });
    QVERIFY(QMetaObject::invokeMethod(layer, "setGrid", Q_ARG(QVariant, QVariant(true))));
    QTRY_COMPARE(tile->width(), pip->width());
    QVERIFY(tile->x() > 0 || tile->y() > 0);
}

void VideoTileTest::_statusPillRegistersAsAnObstacleOwnedByThePip()
{
    QQuickView view;
    const std::unique_ptr<QQmlPropertyMap> globals(QQmlPropertyMap::create());
    globals->insert(QStringLiteral("activeVehicle"), QVariant());
    view.engine()->rootContext()->setContextProperty(QStringLiteral("globals"), globals.get());
    QVERIFY(loadTestView(view, QStringLiteral("qrc:/unittest/StatusPillTest.qml")));

    QQuickItem *const pill = findNamed(view, QStringLiteral("videoStatusPill"));
    QQuickItem *const pip  = findNamed(view, QStringLiteral("pip"));
    QVERIFY(pill && pip);

    const QVariantList statics = view.rootObject()->property("registeredStatics").toList();
    const QVariantList owners  = view.rootObject()->property("registeredOwners").toList();
    QCOMPARE(statics.count(), 1);
    QCOMPARE(statics.first().value<QQuickItem*>(), pill);

    QCOMPARE(owners.count(), 1);
    QCOMPARE(owners.first().value<QQuickItem*>(), pip);
}

UT_REGISTER_TEST(VideoTileTest, TestLabel::Unit)
