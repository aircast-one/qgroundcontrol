#include "VideoTileTest.h"
#include "QuickInteractionTestHelpers.h"

#include <QtQml/QQmlContext>
#include <QtQml/QQmlPropertyMap>

#include <QtCore/QRegularExpression>
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
    const QVariant savedCameras = videoSettings->cameras()->rawValue();
    const QVariant savedActive = videoSettings->activeVideoSource()->rawValue();
    const QVariant savedMultiView = videoSettings->multiViewEnabled()->rawValue();

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

    videoSettings->cameras()->setRawValue(savedCameras);
    videoSettings->activeVideoSource()->setRawValue(savedActive);
    videoSettings->multiViewEnabled()->setRawValue(savedMultiView);
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

void VideoTileTest::_onlyThePipCameraGetsATile()
{
    clearQmlGlobalSettings({"VideoRailGrid"});

    VideoSettings* const videoSettings = SettingsManager::instance()->videoSettings();
    const QVariant savedCameras = videoSettings->cameras()->rawValue();
    const QVariant savedActive = videoSettings->activeVideoSource()->rawValue();
    const QVariant savedMultiView = videoSettings->multiViewEnabled()->rawValue();

    videoSettings->activeVideoSource()->setRawValue(0);
    videoSettings->cameras()->setRawValue(QStringLiteral(R"([
        {"name":"Cam1","source":"RTSP Video Stream","url":"rtsp://127.0.0.1/1"},
        {"name":"Cam2","source":"RTSP Video Stream","url":"rtsp://127.0.0.1/2"},
        {"name":"Cam3","source":"RTSP Video Stream","url":"rtsp://127.0.0.1/3"},
        {"name":"Cam4","source":"RTSP Video Stream","url":"rtsp://127.0.0.1/4"},
        {"name":"Cam5","source":"RTSP Video Stream","url":"rtsp://127.0.0.1/5"}])"));
    videoSettings->multiViewEnabled()->setRawValue(true);

    const std::unique_ptr<QQmlPropertyMap> globals(QQmlPropertyMap::create());
    QQuickView view;
    QVERIFY(loadVideoView(view, *globals));

    QQuickItem* layer = findNamed(view, QStringLiteral("tiles"));
    QQuickItem* tile = findTile(view, 2);
    QVERIFY(layer);
    QVERIFY(tile);
    QVERIFY(tile->isVisible());
    QVERIFY(!findTile(view, 3));
    QVERIFY(!findTile(view, 5));

    QVERIFY(QMetaObject::invokeMethod(layer, "setGrid", Q_ARG(QVariant, QVariant(true))));
    QVERIFY(!findTile(view, 3));

    QQuickItem* pip = findNamed(view, QStringLiteral("pip"));
    QTRY_COMPARE(tile->width(), pip->width());
    QVERIFY(tile->x() > 0 || tile->y() > 0);

    QVERIFY(QMetaObject::invokeMethod(layer, "setGrid", Q_ARG(QVariant, QVariant(false))));
    videoSettings->cameras()->setRawValue(savedCameras);
    videoSettings->activeVideoSource()->setRawValue(savedActive);
    videoSettings->multiViewEnabled()->setRawValue(savedMultiView);
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
