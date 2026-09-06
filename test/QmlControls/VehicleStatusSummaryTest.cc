/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "VehicleStatusSummaryTest.h"
#include "QuickInteractionTestHelpers.h"

#include <QtTest/QTest>
#include <QtQuick/QQuickView>

namespace {

const QString kTestView = QStringLiteral("qrc:/unittest/VehicleStatusSummaryTest.qml");

void setSensors(QObject *summary, const QStringList &names, const QList<bool> &healthy, const QList<bool> &enabled)
{
    const auto toVariants = [](const QList<bool> &values) {
        QVariantList list;
        for (const bool value : values) {
            list.append(value);
        }
        return list;
    };

    summary->setProperty("sensorNames", names);
    summary->setProperty("sensorHealthy", toVariants(healthy));
    summary->setProperty("sensorEnabled", toVariants(enabled));
}

}

void VehicleStatusSummaryTest::_faultsAndDisabledSensorsAreCountedApart()
{
    QQuickView view;
    QVERIFY(loadTestView(view, kTestView));
    QObject *const summary = view.rootObject();

    setSensors(summary,
               { "GPS", "Gyro", "Accelerometer", "Logging" },
               { false, true, true, false },
               { true,  true, true, false });

    QCOMPARE(summary->property("faultList").toString(), QStringLiteral("GPS"));
    QCOMPARE(summary->property("disabledList").toString(), QStringLiteral("Logging"));
    QCOMPARE(summary->property("normalCount").toInt(), 2);
    QCOMPARE(summary->property("fault").toBool(), true);
    QCOMPARE(summary->property("nominal").toBool(), false);

    setSensors(summary,
               { "GPS", "Gyro", "Logging" },
               { true, true, false },
               { true, true, false });

    QCOMPARE(summary->property("faultList").toString(), QString());
    QCOMPARE(summary->property("fault").toBool(), false);
    QCOMPARE(summary->property("caution").toBool(), true);
    QCOMPARE(summary->property("nominal").toBool(), false);
}

void VehicleStatusSummaryTest::_healthChecksDecideTheStatusWhenSupported()
{
    QQuickView view;
    QVERIFY(loadTestView(view, kTestView));
    QObject *const summary = view.rootObject();

    summary->setProperty("healthChecksSupported", true);
    summary->setProperty("canArm", true);
    summary->setProperty("hasWarningsOrErrors", false);
    QCOMPARE(summary->property("nominal").toBool(), true);

    summary->setProperty("hasWarningsOrErrors", true);
    QCOMPARE(summary->property("caution").toBool(), true);
    QCOMPARE(summary->property("fault").toBool(), false);
    QCOMPARE(summary->property("nominal").toBool(), false);

    summary->setProperty("canArm", false);
    QCOMPARE(summary->property("fault").toBool(), true);
    QCOMPARE(summary->property("nominal").toBool(), false);
}

void VehicleStatusSummaryTest::_noVehicleDataReadsAsNominal()
{
    QQuickView view;
    QVERIFY(loadTestView(view, kTestView));
    QObject *const summary = view.rootObject();

    QCOMPARE(summary->property("normalCount").toInt(), 0);
    QCOMPARE(summary->property("faultList").toString(), QString());
    QCOMPARE(summary->property("nominal").toBool(), true);
}
