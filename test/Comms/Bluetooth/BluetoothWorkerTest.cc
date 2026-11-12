#include "BluetoothWorkerTest.h"

#include <QtBluetooth/QBluetoothServiceDiscoveryAgent>
#include <QtCore/QRegularExpression>
#include <QtCore/QTimer>
#include <QtTest/QSignalSpy>

#include "BluetoothBleWorker.h"
#include "BluetoothClassicWorker.h"
#include "BluetoothConfiguration.h"
#include "BluetoothWorker.h"

static QTimer* _findServiceDiscoveryTimer(const BluetoothWorker* worker)
{
    for (QTimer* timer : worker->findChildren<QTimer*>()) {
        if (timer && timer->isSingleShot() && (timer->interval() == 30000)) {
            return timer;
        }
    }

    return nullptr;
}

void BluetoothWorkerTest::init()
{
    UnitTest::init();

    ignoreLogMessage("default", QtWarningMsg,
                     QRegularExpression(QStringLiteral("Cannot find a compatible running Bluez")));
    ignoreLogMessage("qt.bluetooth", QtWarningMsg,
                     QRegularExpression(QStringLiteral("LE controller has invalid adapter")));
    ignoreLogMessage("qt.bluetooth.bluez", QtInfoMsg, QRegularExpression(QStringLiteral("Missing CAP_NET_ADMIN")));
    ignoreLogMessage(
        "qt.bluetooth.bluez", QtWarningMsg,
        QRegularExpression(QStringLiteral(
            "Cannot open HCI socket|Cannot determine bluetoothd version|"
            "Disabling Qt Bluetooth LE feature|Cannot find Bluez 5 adapter|HCIGETCONNLIST|Cannot find hci dev")));
    ignoreLogMessage("Comms.Bluetooth.BluetoothBleWorker", QtWarningMsg,
                     QRegularExpression(QStringLiteral("BLE Controller error")));
    ignoreLogMessage("Comms.Bluetooth.BluetoothClassicWorker", QtWarningMsg,
                     QRegularExpression(QStringLiteral(
                         "Cannot find valid Bluetooth adapter|Socket error|No suitable classic service found")));
}

void BluetoothWorkerTest::_testFactoryCreatesClassicWorker()
{
    BluetoothConfiguration config("TestBT_factoryClassic");
    config.setMode(BluetoothConfiguration::BluetoothMode::ModeClassic);

    std::unique_ptr<BluetoothWorker> worker(BluetoothWorker::create(&config));
    QVERIFY(worker);
    QVERIFY(qobject_cast<BluetoothClassicWorker*>(worker.get()));
}

void BluetoothWorkerTest::_testFactoryCreatesBleWorker()
{
    BluetoothConfiguration config("TestBT_factoryBle");
    config.setMode(BluetoothConfiguration::BluetoothMode::ModeLowEnergy);

    std::unique_ptr<BluetoothWorker> worker(BluetoothWorker::create(&config));
    QVERIFY(worker);
    QVERIFY(qobject_cast<BluetoothBleWorker*>(worker.get()));
}

void BluetoothWorkerTest::_testInitialState()
{
    BluetoothConfiguration config("TestBT_initialState");

    std::unique_ptr<BluetoothWorker> worker(BluetoothWorker::create(&config));
    QVERIFY(!worker->isConnected());
}

void BluetoothWorkerTest::_testWriteEmptyDataEmitsNothing()
{
    BluetoothConfiguration config("TestBT_writeEmpty");

    std::unique_ptr<BluetoothWorker> worker(BluetoothWorker::create(&config));
    QSignalSpy errorSpy(worker.get(), &BluetoothWorker::errorOccurred);
    QSignalSpy dataSpy(worker.get(), &BluetoothWorker::dataSent);

    expectLogMessage("Comms.Bluetooth.BluetoothWorker", QtWarningMsg,
                     QRegularExpression("Write called with empty data"));
    worker->writeData(QByteArray());
    verifyExpectedLogMessage();

    QCOMPARE(errorSpy.count(), 0);
    QCOMPARE(dataSpy.count(), 0);
}

void BluetoothWorkerTest::_testWriteWhileDisconnectedEmitsError()
{
    BluetoothConfiguration config("TestBT_writeDisconnected");

    std::unique_ptr<BluetoothWorker> worker(BluetoothWorker::create(&config));
    QSignalSpy errorSpy(worker.get(), &BluetoothWorker::errorOccurred);

    worker->writeData(QByteArray("hello"));

    QCOMPARE(errorSpy.count(), 1);
    QVERIFY(errorSpy.first().first().toString().contains("not connected"));
}

void BluetoothWorkerTest::_testConnectWhileConnectedWarns()
{
    BluetoothConfiguration config("TestBT_doubleConnect");

    config.setMode(BluetoothConfiguration::BluetoothMode::ModeLowEnergy);
    std::unique_ptr<BluetoothWorker> worker(BluetoothWorker::create(&config));

    worker->connectLink();
    worker->connectLink();

    QVERIFY2(true, "double connectLink must not crash");
}

void BluetoothWorkerTest::_testDisconnectResetsReconnectState()
{
    BluetoothConfiguration config("TestBT_disconnectReset");

    std::unique_ptr<BluetoothWorker> worker(BluetoothWorker::create(&config));

    worker->connectLink();

    worker->disconnectLink();

    QVERIFY(!worker->isConnected());
}

void BluetoothWorkerTest::_testClassicDiscoveryTimerStopsOnFinished()
{
    BluetoothConfiguration config("TestBT_classicTimerFinished");
    config.setMode(BluetoothConfiguration::BluetoothMode::ModeClassic);

    std::unique_ptr<BluetoothWorker> worker(BluetoothWorker::create(&config));
    auto* classicWorker = qobject_cast<BluetoothClassicWorker*>(worker.get());
    QVERIFY(classicWorker);

    QTimer* serviceTimer = _findServiceDiscoveryTimer(worker.get());
    QVERIFY(serviceTimer);
    serviceTimer->start();
    QVERIFY(serviceTimer->isActive());

    expectLogMessage("Comms.Bluetooth.BluetoothClassicWorker", QtWarningMsg,
                     QRegularExpression("No suitable classic service found"));
    QVERIFY(QMetaObject::invokeMethod(classicWorker, "_onClassicServiceDiscoveryFinished", Qt::DirectConnection));
    verifyExpectedLogMessage();
    QVERIFY(!serviceTimer->isActive());
}

void BluetoothWorkerTest::_testClassicDiscoveryTimerStopsOnCanceled()
{
    BluetoothConfiguration config("TestBT_classicTimerCanceled");
    config.setMode(BluetoothConfiguration::BluetoothMode::ModeClassic);

    std::unique_ptr<BluetoothWorker> worker(BluetoothWorker::create(&config));
    auto* classicWorker = qobject_cast<BluetoothClassicWorker*>(worker.get());
    QVERIFY(classicWorker);

    QTimer* serviceTimer = _findServiceDiscoveryTimer(worker.get());
    QVERIFY(serviceTimer);
    serviceTimer->start();
    QVERIFY(serviceTimer->isActive());

    QVERIFY(QMetaObject::invokeMethod(classicWorker, "_onClassicServiceDiscoveryCanceled", Qt::DirectConnection));
    QVERIFY(!serviceTimer->isActive());
}

void BluetoothWorkerTest::_testClassicDiscoveryTimerStopsOnError()
{
    qRegisterMetaType<QBluetoothServiceDiscoveryAgent::Error>("QBluetoothServiceDiscoveryAgent::Error");

    BluetoothConfiguration config("TestBT_classicTimerError");
    config.setMode(BluetoothConfiguration::BluetoothMode::ModeClassic);

    std::unique_ptr<BluetoothWorker> worker(BluetoothWorker::create(&config));
    auto* classicWorker = qobject_cast<BluetoothClassicWorker*>(worker.get());
    QVERIFY(classicWorker);

    QTimer* serviceTimer = _findServiceDiscoveryTimer(worker.get());
    QVERIFY(serviceTimer);
    serviceTimer->start();
    QVERIFY(serviceTimer->isActive());

    expectLogMessage("Comms.Bluetooth.BluetoothClassicWorker", QtWarningMsg,
                     QRegularExpression("Service discovery error:"));
    QVERIFY(QMetaObject::invokeMethod(
        classicWorker, "_onClassicServiceDiscoveryError", Qt::DirectConnection,
        Q_ARG(QBluetoothServiceDiscoveryAgent::Error, QBluetoothServiceDiscoveryAgent::Error::UnknownError)));
    verifyExpectedLogMessage();
    QVERIFY(!serviceTimer->isActive());
}

void BluetoothWorkerTest::_testDestroyClassicWorker()
{
    BluetoothConfiguration config("TestBT_destroyClassic");
    config.setMode(BluetoothConfiguration::BluetoothMode::ModeClassic);

    {
        std::unique_ptr<BluetoothWorker> worker(BluetoothWorker::create(&config));
        worker->setupConnection();
    }
    QVERIFY2(true, "classic worker destruction must not crash");
}

void BluetoothWorkerTest::_testDestroyBleWorker()
{
    BluetoothConfiguration config("TestBT_destroyBle");
    config.setMode(BluetoothConfiguration::BluetoothMode::ModeLowEnergy);

    {
        std::unique_ptr<BluetoothWorker> worker(BluetoothWorker::create(&config));
        worker->setupConnection();
    }
    QVERIFY2(true, "BLE worker destruction must not crash");
}

UT_REGISTER_TEST(BluetoothWorkerTest, TestLabel::Unit, TestLabel::Comms)
