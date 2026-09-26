#include "QmlUITestBase.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QElapsedTimer>
#include <QtCore/QEventLoop>
#include <QtCore/QtMath>
#include <QtCore/QRegularExpression>
#include <QtCore/QScopeGuard>
#include <QtCore/QVariant>
#include <QtQml/QQmlApplicationEngine>
#include <QtQml/QQmlIncubationController>
#include <QtQuick/QQuickItem>
#include <QtQuick/QQuickWindow>
#include <QtQuickControls2/QQuickStyle>
#include <QtTest/QSignalSpy>
#include <QtTest/QTest>

#include "AppSettings.h"
#include "ColoredSvgImageProvider.h"
#include "MAVLinkProtocol.h"
#include "MockLink.h"
#include "MultiVehicleManager.h"
#include "QGCApplication.h"
#include "QGCCorePlugin.h"
#include "QGCFileDialogController.h"
#include "QGCImageProvider.h"
#include "SettingsManager.h"
#include "Vehicle.h"

static constexpr int kSettleDrainMs = 100;

static QQuickItem *findVisibleItemImmediate(QQuickItem *root, const QString &objectName)
{
    if (!root || !root->isVisible()) {
        return nullptr;
    }
    if (root->objectName() == objectName) {
        return root;
    }
    const auto children = root->childItems();
    for (auto *child : children) {
        if (auto *found = findVisibleItemImmediate(child, objectName)) {
            return found;
        }
    }
    return nullptr;
}

QQuickItem *QmlUITestBase::findItem(QQuickItem *root, const QString &objectName)
{
    if (!root) {
        return nullptr;
    }
    if (root->objectName() == objectName) {
        return root;
    }
    const QList<QQuickItem *> children = root->childItems();
    for (QQuickItem *child : children) {
        if (QQuickItem *found = findItem(child, objectName)) {
            return found;
        }
    }
    return nullptr;
}

QQuickItem *QmlUITestBase::findVisibleItem(QQuickItem *root, const QString &objectName, int timeoutMs)
{
    constexpr int pollIntervalMs = 50;
    int elapsed = 0;
    while (elapsed <= timeoutMs) {
        if (auto *item = findVisibleItemImmediate(root, objectName)) {
            return item;
        }
        if (elapsed >= timeoutMs) {
            break;
        }
        QTest::qWait(pollIntervalMs);
        elapsed += pollIntervalMs;
    }
    return nullptr;
}

void QmlUITestBase::startUI()
{
    static bool s_styleSet = false;
    if (!s_styleSet) {
        QQuickStyle::setStyle("Basic");
        s_styleSet = true;
    }
    QGCCorePlugin::instance()->init();
    MAVLinkProtocol::instance()->init();
    MultiVehicleManager::instance()->init();

    AppSettings *appSettings = SettingsManager::instance()->appSettings();
    const QList<int> promptIds = QGCCorePlugin::instance()->firstRunPromptStdIds();
    for (int id : promptIds) {
        appSettings->firstRunPromptIdsMarkIdAsShown(id);
    }

    QVERIFY2(QGCCorePlugin::instance(), "Core plugin not available");
    QGCCorePlugin::instance()->setProperty("showAdvancedUI", true);
    QVERIFY2(QGCCorePlugin::instance()->showAdvancedUI(), "Test requires Advanced UI mode");

    ignoreLogMessage("default", QtWarningMsg,
                     QRegularExpression(QStringLiteral("This plugin does not support propagateSizeHints")));
    ignoreLogMessage("qt.qpa.fonts", QtWarningMsg,
                     QRegularExpression(QStringLiteral("Populating font family aliases")));
    ignoreLogMessage("default", QtWarningMsg,
                     QRegularExpression(QStringLiteral("QRhiGles2")));
    ignoreLogMessage("default", QtInfoMsg,
                     QRegularExpression(QStringLiteral("Object or context destroyed during incubation")));
    ignoreLogMessage("default", QtWarningMsg,
                     QRegularExpression(QStringLiteral("in the process of being created at engine destruction")));

    ignoreLogMessage("default", QtWarningMsg,
                     QRegularExpression(QStringLiteral("items in the process of being created at engine destruction")));

#ifdef QT_DEBUG
    ignoreLogMessage("default", QtWarningMsg,
                     QRegularExpression(QStringLiteral("Access to camera not granted")));
#endif

    _engine = QGCCorePlugin::instance()->createQmlApplicationEngine(this);
    QVERIFY(_engine);

    _engine->addImageProvider(QStringLiteral("QGCImages"),               new QGCImageProvider());
    _engine->addImageProvider(QLatin1String(ColoredSvgImageProvider::ProviderId), new ColoredSvgImageProvider());

    _engine->load(QUrl(QStringLiteral("qrc:/qml/QGroundControl/MainWindow.qml")));
    QVERIFY(!_engine->rootObjects().isEmpty());

    qgcApp()->setQmlAppEngine(_engine);

    _window = qobject_cast<QQuickWindow *>(_engine->rootObjects().first());
    QVERIFY(_window);

    _window->resize(1600, 1000);
    QVERIFY(QTest::qWaitForWindowExposed(_window));

    _rootItem = _window->contentItem();
}

void QmlUITestBase::closeUIWindow()
{
    if (_window) {
        _window->close();
        (void) QTest::qWaitFor([this] { return !_window->isVisible(); }, TestTimeout::shortMs());
        QElapsedTimer settle;
        settle.start();
        while (settle.elapsed() < kSettleDrainMs) {
            QCoreApplication::processEvents(QEventLoop::AllEvents, 20);
        }
    }
}

void QmlUITestBase::destroyUIEngine()
{
    if (_engine) {
        if (QQmlIncubationController *controller = _engine->incubationController()) {
            QElapsedTimer drainTimer;
            drainTimer.start();
            while ((controller->incubatingObjectCount() > 0) && (drainTimer.elapsed() < 2000)) {
                controller->incubateFor(50);
                QCoreApplication::processEvents();
            }
        }

        QElapsedTimer gcSettle;
        gcSettle.start();
        while (gcSettle.elapsed() < kSettleDrainMs) {
            _engine->collectGarbage();
            QCoreApplication::processEvents(QEventLoop::AllEvents, 10);
        }

        _engine->collectGarbage();
        QCoreApplication::processEvents();
        _engine->clearComponentCache();
        QCoreApplication::processEvents();

        delete _window;
        _window = nullptr;
        QCoreApplication::processEvents();
    }
    qgcApp()->setQmlAppEngine(nullptr);
    QGCCorePlugin::instance()->destroyQmlApplicationEngine(_engine);
    _engine   = nullptr;
    _window   = nullptr;
    _rootItem = nullptr;
    QCoreApplication::processEvents();
}

void QmlUITestBase::stopUI()
{
    closeUIWindow();
    destroyUIEngine();
}

void QmlUITestBase::_verifyFileDialogTestHookConsumed()
{
    if (QGCFileDialogController::testHookArmed()) {
        QGCFileDialogController::takeTestNextFile();
        QTest::qFail("file dialog test hook was armed but never consumed by a dialog", __FILE__, __LINE__);
    }
}

bool QmlUITestBase::clickButton(const QString &objectName)
{
    QQuickItem *btn = findVisibleItem(_rootItem, objectName);
    if (!btn) {
        return false;
    }
    return _clickItemAt(btn, 0.5, 0.5, objectName);
}

bool QmlUITestBase::_clickItemAt(QQuickItem *item, qreal fractionX, qreal fractionY, const QString &objectName)
{
    const QPointer<QQuickItem> guarded(item);

    const auto scenePoint = [&]() -> QPointF {
        for (QQuickItem *ancestor = guarded; ancestor; ancestor = ancestor->parentItem()) {
            ancestor->ensurePolished();
        }
        return guarded->mapToScene(QPointF(guarded->width() * fractionX, guarded->height() * fractionY));
    };
    QPointF lastPos(qQNaN(), qQNaN());
    const bool settled = waitForCondition(
        [&] {
            if (!guarded) {
                return false;
            }
            const QPointF pos = scenePoint();
            const bool stable = (pos == lastPos);
            lastPos = pos;
            return stable;
        },
        TestTimeout::shortMs(),
        QStringLiteral("%1 position settled").arg(objectName));
    if (!guarded) {
        QTest::qFail(qPrintable(QStringLiteral("%1 destroyed while waiting to click it").arg(objectName)),
                     __FILE__, __LINE__);
        return false;
    }
    if (!settled) {
        return false;
    }

    const QPointF scenePos = scenePoint();
    if (scenePos.x() < 0 || scenePos.x() >= _window->width()
        || scenePos.y() < 0 || scenePos.y() >= _window->height()) {
        QTest::qFail(qPrintable(QStringLiteral("%1 click point (%2, %3) is outside the window (%4x%5)")
                                    .arg(objectName).arg(scenePos.x()).arg(scenePos.y())
                                    .arg(_window->width()).arg(_window->height())),
                     __FILE__, __LINE__);
        return false;
    }
    const QPoint clickPoint(qFloor(scenePos.x()), qFloor(scenePos.y()));

    QTest::mouseClick(_window, Qt::LeftButton, Qt::NoModifier, clickPoint);
    return true;
}

bool QmlUITestBase::clickItemFraction(const QString &objectName, qreal fractionX, qreal fractionY)
{
    if (!qIsFinite(fractionX) || !qIsFinite(fractionY)
        || (fractionX < 0) || (fractionX > 1) || (fractionY < 0) || (fractionY > 1)) {
        QTest::qFail(qPrintable(QStringLiteral("clickItemFraction: fractions out of [0,1]: (%1, %2)")
                                    .arg(fractionX).arg(fractionY)),
                     __FILE__, __LINE__);
        return false;
    }
    QQuickItem *item = findVisibleItem(_rootItem, objectName);
    if (!item) {
        return false;
    }
    return _clickItemAt(item, fractionX, fractionY, objectName);
}

QQuickItem *QmlUITestBase::findVisibleItemScrolled(const QString &objectName, const QString &flickableObjectName)
{
    QQuickItem *item = findVisibleItem(_rootItem, objectName, 500);
    if (item) {
        return scrollIntoView(item, flickableObjectName) ? item : nullptr;
    }

    QQuickItem *flickable = findVisibleItem(_rootItem, flickableObjectName);
    if (!flickable) {
        return nullptr;
    }

    const double viewportHeight = flickable->height();
    if (viewportHeight <= 0) {
        return nullptr;
    }
    constexpr int kMaxScrollSteps = 100;
    double y = 0;
    for (int step = 0; step < kMaxScrollSteps; step++, y += viewportHeight) {
        const double maxContentY =
            qMax(0.0, flickable->property("contentHeight").toDouble() - viewportHeight);
        const double clampedY = qMin(y, maxContentY);
        flickable->setProperty("contentY", clampedY);
        item = findVisibleItem(_rootItem, objectName, 100);
        if (item) {
            return scrollIntoView(item, flickableObjectName) ? item : nullptr;
        }
        if (clampedY >= maxContentY) {
            break;
        }
    }
    return nullptr;
}

bool QmlUITestBase::clickButtonScrolled(const QString &objectName, const QString &flickableObjectName)
{
    if (!findVisibleItemScrolled(objectName, flickableObjectName)) {
        return false;
    }
    return clickButton(objectName);
}

static bool _toolPanelAtRest(QQuickItem *panel)
{
    const QQuickItem *const drawer = panel->parentItem();
    const bool floating = drawer->property("floating").toBool();
    const qreal targetWidth = drawer->property(floating ? "_panelWidth" : "_safeWidth").toReal();
    const qreal targetHeight = drawer->property(floating ? "_panelHeight" : "_safeHeight").toReal();
    return qFuzzyCompare(panel->width(), targetWidth) && qFuzzyCompare(panel->height(), targetHeight);
}

bool QmlUITestBase::_waitForToolPanel()
{
    QQuickItem *const panel = findVisibleItem(_rootItem, QStringLiteral("toolPanel"), TestTimeout::shortMs());
    return panel && waitForCondition([panel] { return _toolPanelAtRest(panel); }, TestTimeout::shortMs(),
                                     QStringLiteral("tool panel at rest"));
}

bool QmlUITestBase::openPlanView()
{
    if (!clickButton(QStringLiteral("viewSwitchOption1"))) {
        QTest::qFail("Plan option of the view switch not found", __FILE__, __LINE__);
        return false;
    }
    return waitForCondition([this] { return !_window->property("flyViewActive").toBool(); }, TestTimeout::shortMs(),
                            QStringLiteral("plan view active"));
}

bool QmlUITestBase::openSettings()
{
    if (!clickButton(QStringLiteral("settingsButton"))) {
        QTest::qFail("Toolbar settings button not found", __FILE__, __LINE__);
        return false;
    }
    if (!findVisibleItem(_rootItem, QStringLiteral("appSettingsView"), TestTimeout::shortMs())) {
        QTest::qFail("Settings tool did not open", __FILE__, __LINE__);
        return false;
    }
    return _waitForToolPanel();
}

bool QmlUITestBase::openSettingsPage(const QString &pageName)
{
    if (!findVisibleItem(_rootItem, QStringLiteral("appSettingsView"), 0) && !openSettings()) {
        return false;
    }
    const QString buttonName = QStringLiteral("settingsPage") + QString(pageName).remove(QLatin1Char(' '));
    if (!clickButtonScrolled(buttonName, QStringLiteral("settingsList"))) {
        QTest::qFail(qPrintable(QStringLiteral("Settings page entry not found: %1").arg(buttonName)), __FILE__, __LINE__);
        return false;
    }
    QQuickItem *const settingsView = findVisibleItem(_rootItem, QStringLiteral("appSettingsView"));
    QQuickItem *const loader = findVisibleItem(_rootItem, QStringLiteral("settingsPageLoader"));
    return settingsView && loader && waitForCondition(
        [settingsView, loader, pageName] {
            return (settingsView->property("_pageTitle").toString() == pageName)
                && (loader->property("status").toInt() == 1)
                && loader->property("item").value<QObject *>();
        },
        TestTimeout::shortMs(), QStringLiteral("settings page %1 loaded").arg(pageName));
}

bool QmlUITestBase::openAnalyzeTools()
{
    if (!clickButton(QStringLiteral("analyzeButton"))) {
        QTest::qFail("Toolbar analyze button not found", __FILE__, __LINE__);
        return false;
    }
    return _waitForToolPanel();
}

bool QmlUITestBase::openVehicleSetup()
{
    if (findVisibleItem(_rootItem, QStringLiteral("vehicleSetupView"), 0)) {
        return true;
    }
    if (!clickButton(QStringLiteral("mainStatusPill"))) {
        QTest::qFail("Main status pill not found", __FILE__, __LINE__);
        return false;
    }
    if (!findVisibleItem(_rootItem, QStringLiteral("vehicleSetupItem"), TestTimeout::shortMs())
        || !clickButton(QStringLiteral("vehicleSetupItem"))) {
        QTest::qFail("Vehicle Setup entry not found in the main status drop-down", __FILE__, __LINE__);
        return false;
    }
    if (!findVisibleItem(_rootItem, QStringLiteral("vehicleSetupView"), TestTimeout::shortMs())) {
        QTest::qFail("Vehicle Setup did not open", __FILE__, __LINE__);
        return false;
    }
    QObject *const drawer = _window->findChild<QObject *>(QStringLiteral("indicatorDrawer"));
    return drawer
        && waitForCondition([drawer] { return !drawer->property("visible").toBool(); }, TestTimeout::shortMs(),
                            QStringLiteral("main status drop-down closed"))
        && _waitForToolPanel();
}

static QQuickItem *_findVisibleItemWithText(QQuickItem *root, const QString &objectName, const QString &textSubstring)
{
    if (!root || !root->isVisible()) {
        return nullptr;
    }
    if (root->objectName() == objectName) {
        const QVariant textProp = root->property("text");
        if (textProp.isValid() && textProp.toString().contains(textSubstring)) {
            return root;
        }
    }
    const auto children = root->childItems();
    for (auto *child : children) {
        if (auto *found = _findVisibleItemWithText(child, objectName, textSubstring)) {
            return found;
        }
    }
    return nullptr;
}

bool QmlUITestBase::dialogVisible(const QString &textSubstring)
{
    return _findVisibleItemWithText(_rootItem, QStringLiteral("popupDialog_title"), textSubstring)
        || _findVisibleItemWithText(_rootItem, QStringLiteral("popupDialog_text"), textSubstring);
}

bool QmlUITestBase::waitForDialog(const QString &textSubstring, int timeoutMs)
{
    return waitForCondition([this, &textSubstring] { return dialogVisible(textSubstring); },
                            timeoutMs, QStringLiteral("dialog '%1'").arg(textSubstring));
}

bool QmlUITestBase::_clickDialogButton(const QString &buttonName, int timeoutMs)
{
    const QPointer<QQuickItem> button = findVisibleItem(_rootItem, buttonName, timeoutMs);
    if (!button || !_clickItemAt(button, 0.5, 0.5, buttonName)) {
        return false;
    }
    return waitForCondition([button] { return !button || !button->isVisible(); }, TestTimeout::shortMs(),
                            QStringLiteral("%1 dialog closed").arg(buttonName));
}

bool QmlUITestBase::acceptDialog(int timeoutMs)
{
    return _clickDialogButton(QStringLiteral("popupDialog_acceptButton"), timeoutMs);
}

bool QmlUITestBase::rejectDialog(int timeoutMs)
{
    return _clickDialogButton(QStringLiteral("popupDialog_rejectButton"), timeoutMs);
}

static QString _displayValue(const QVariant &value)
{
    if (value.typeId() == QMetaType::QString) {
        return QStringLiteral("'%1'").arg(value.toString());
    }
    return value.toString();
}

bool QmlUITestBase::_verifyItemProperty(const QString &objectName, const char *propertyName,
                                        const QVariant &expectedValue, const QString &context)
{
    QQuickItem *item = findVisibleItem(_rootItem, objectName, 2000);
    if (!item) {
        QTest::qFail(qPrintable(QStringLiteral("%1: item not found: %2").arg(context, objectName)),
                     __FILE__, __LINE__);
        return false;
    }

    if (!item->property(propertyName).isValid()) {
        QTest::qFail(qPrintable(QStringLiteral("%1: %2 has no '%3' property")
                                    .arg(context, objectName, QLatin1String(propertyName))),
                     __FILE__, __LINE__);
        return false;
    }

    const QPointer<QQuickItem> guardedItem(item);

    const bool matched = waitForCondition(
        [guardedItem, propertyName, expectedValue] {
            return guardedItem && (guardedItem->property(propertyName) == expectedValue);
        },
        2000,
        QStringLiteral("%1 %2 == %3").arg(objectName, QLatin1String(propertyName), _displayValue(expectedValue)));
    if (!matched) {
        QTest::qFail(qPrintable(QStringLiteral("%1: %2 expected %3=%4 but was %5")
                                    .arg(context, objectName, QLatin1String(propertyName),
                                         _displayValue(expectedValue),
                                         guardedItem ? _displayValue(guardedItem->property(propertyName))
                                                     : QStringLiteral("<item destroyed>"))),
                     __FILE__, __LINE__);
        return false;
    }
    return true;
}

bool QmlUITestBase::verifyEnabled(const QString &objectName, bool expectedEnabled, const QString &context)
{
    return _verifyItemProperty(objectName, "enabled", expectedEnabled, context);
}

bool QmlUITestBase::verifyProperty(const QString &objectName, const char *propertyName,
                                   const QVariant &expectedValue, const QString &context)
{
    return _verifyItemProperty(objectName, propertyName, expectedValue, context);
}

bool QmlUITestBase::verifyVisibility(const QString &objectName, bool expectedVisible, const QString &context)
{
    const bool result = waitForCondition(
        [this, objectName, expectedVisible] {
            return (findVisibleItem(_rootItem, objectName, 0) != nullptr) == expectedVisible;
        },
        2000,
        QStringLiteral("%1 %2").arg(objectName, expectedVisible ? QStringLiteral("visible") : QStringLiteral("absent")));
    if (!result) {
        QTest::qFail(qPrintable(QStringLiteral("%1: %2 expected %3")
                                    .arg(context, objectName,
                                         expectedVisible ? QStringLiteral("visible") : QStringLiteral("absent"))),
                     __FILE__, __LINE__);
    }
    return result;
}

bool QmlUITestBase::verifyPrimary(const QString &objectName, bool expectedPrimary, const QString &context)
{
    return _verifyItemProperty(objectName, "primary", expectedPrimary, context);
}

bool QmlUITestBase::verifyChecked(const QString &objectName, bool expectedChecked, const QString &context)
{
    return _verifyItemProperty(objectName, "checked", expectedChecked, context);
}

bool QmlUITestBase::verifyText(const QString &objectName, const QString &expectedText, const QString &context)
{
    return _verifyItemProperty(objectName, "text", expectedText, context);
}

bool QmlUITestBase::scrollIntoView(QQuickItem *item, const QString &flickableObjectName)
{
    if (!item || !_rootItem || !_window) {
        return false;
    }

    QQuickItem *flickable = findVisibleItem(_rootItem, flickableObjectName);
    if (!flickable) {
        return false;
    }

    const QPointer<QQuickItem> guardedItem(item);
    const QPointer<QQuickItem> guardedFlickable(flickable);
    QPointF lastCenter(qQNaN(), qQNaN());
    const bool settled = waitForCondition(
        [&] {
            if (!guardedItem || !guardedFlickable) {
                return false;
            }
            for (QQuickItem *ancestor = guardedItem; ancestor; ancestor = ancestor->parentItem()) {
                ancestor->ensurePolished();
            }
            const QPointF sceneCenter =
                guardedItem->mapToScene(QPointF(guardedItem->width() / 2, guardedItem->height() / 2));
            const QRectF flickableViewport(guardedFlickable->mapToScene(QPointF(0, 0)),
                                           QSizeF(guardedFlickable->width(), guardedFlickable->height()));
            const QRectF windowRect(0, 0, _window->width(), _window->height());
            const QRectF clickableViewport = flickableViewport.intersected(windowRect).adjusted(1, 1, -1, -1);

            const bool stable = (sceneCenter == lastCenter);
            lastCenter = sceneCenter;
            if (clickableViewport.contains(sceneCenter)) {
                return stable;
            }

            const QPointF itemInFlickable = guardedItem->mapToItem(guardedFlickable, QPointF(0, 0));
            const double currentContentY = guardedFlickable->property("contentY").toDouble();
            const double absoluteY = itemInFlickable.y() + currentContentY;
            const double targetY = absoluteY + guardedItem->height() / 2.0 - guardedFlickable->height() / 2.0;
            const double maxContentY =
                guardedFlickable->property("contentHeight").toDouble() - guardedFlickable->height();
            guardedFlickable->setProperty("contentY", qBound(0.0, targetY, qMax(0.0, maxContentY)));
            return false;
        },
        TestTimeout::shortMs(),
        QStringLiteral("scrollIntoView settled"));
    if (!settled) {
        QTest::qFail(qPrintable(QStringLiteral("scrollIntoView: item never settled inside flickable %1")
                                    .arg(flickableObjectName)),
                     __FILE__, __LINE__);
        return false;
    }
    return true;
}

void QmlUITestBase::runWithMockLink(
    const std::function<MockLink *()> &factory,
    const std::function<void(QPointer<MockLink>, Vehicle *)> &body)
{
    startUI();
    if (QTest::currentTestFailed()) return;

    Vehicle *vehicle = nullptr;
    QPointer<MockLink> mockLink = connectMockLinkAndWaitReady(factory, vehicle);
    if (!mockLink) return;

    const auto cleanup = qScopeGuard([&] {
        disconnectMockLink(mockLink);
        closeUIWindow();
        destroyUIEngine();
    });

    body(mockLink, vehicle);
}

void QmlUITestBase::disconnectMockLink(QPointer<MockLink> mockLink)
{
    if (!mockLink) return;

    QSignalSpy spyDisconnect(MultiVehicleManager::instance(),
                             &MultiVehicleManager::activeVehicleChanged);
    mockLink->disconnect();
    if (spyDisconnect.isValid()) {
        (void)waitForSignal(spyDisconnect, 5000, QStringLiteral("activeVehicleChanged"));
    }
}

QPointer<MockLink> QmlUITestBase::connectMockLinkAndWaitReady(
    const std::function<MockLink *()> &factory,
    Vehicle *&vehicleOut)
{
    vehicleOut = nullptr;

    QSignalSpy spyVehicle(MultiVehicleManager::instance(), &MultiVehicleManager::activeVehicleChanged);
    if (!spyVehicle.isValid()) {
        QTest::qFail("Failed to create spy for activeVehicleChanged", __FILE__, __LINE__);
        return {};
    }

    QPointer<MockLink> mockLink = factory();
    if (!mockLink) {
        QTest::qFail("Failed to start MockLink", __FILE__, __LINE__);
        return {};
    }

    const auto failAndDisconnect = [&](const char *msg) -> QPointer<MockLink> {
        QTest::qFail(msg, __FILE__, __LINE__);
        mockLink->disconnect();
        return {};
    };

    if (!waitForSignal(spyVehicle, 10000, QStringLiteral("activeVehicleChanged"))) {
        return failAndDisconnect("Timeout waiting for vehicle connection");
    }

    Vehicle *vehicle = MultiVehicleManager::instance()->activeVehicle();
    if (!vehicle) {
        return failAndDisconnect("No active vehicle after MockLink connection");
    }

    QSignalSpy spyConnect(vehicle, &Vehicle::initialConnectComplete);
    if (!spyConnect.isValid()) {
        return failAndDisconnect("Failed to create spy for initialConnectComplete");
    }
    if (!vehicle->isInitialConnectComplete()) {
        if (!waitForSignal(spyConnect, 10000, QStringLiteral("initialConnectComplete"))) {
            return failAndDisconnect("Timeout waiting for initial connect");
        }
    }

    QSignalSpy spyParamsReady(MultiVehicleManager::instance(),
                              &MultiVehicleManager::parameterReadyVehicleAvailableChanged);
    if (!spyParamsReady.isValid()) {
        return failAndDisconnect("Failed to create spy for parameterReadyVehicleAvailableChanged");
    }
    if (!MultiVehicleManager::instance()->parameterReadyVehicleAvailable()) {
        if (!waitForSignal(spyParamsReady, 15000,
                           QStringLiteral("parameterReadyVehicleAvailableChanged"))) {
            return failAndDisconnect("Timeout waiting for parameters to be ready");
        }
    }
    if (!MultiVehicleManager::instance()->parameterReadyVehicleAvailable()) {
        return failAndDisconnect("Parameters should be ready after signal");
    }

    vehicleOut = vehicle;
    return mockLink;
}
