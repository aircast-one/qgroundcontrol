#include "TopLevelViewsTest.h"

#include <numeric>

#include <QtQuick/QQuickItem>
#include <QtQuick/QQuickWindow>
#include <QtTest/QTest>

UT_REGISTER_TEST(TopLevelViewsTest, TestLabel::Integration)

static QStringList _visiblePageTitles(QQuickItem *item)
{
    if (!item->isVisible()) {
        return {};
    }
    const QList<QQuickItem *> children = item->childItems();
    return std::accumulate(children.cbegin(), children.cend(),
                           item->objectName().startsWith(QStringLiteral("settingsPage"))
                               ? QStringList{item->property("text").toString()}
                               : QStringList{},
                           [](QStringList titles, QQuickItem *child) { return titles + _visiblePageTitles(child); });
}

void TopLevelViewsTest::_testNavigateViews()
{
    startUI();
    if (QTest::currentTestFailed()) {
        return;
    }

    QVERIFY(_window->property("flyViewActive").toBool());

    QVERIFY(openPlanView());
    QVERIFY(clickButton(QStringLiteral("planViewSwitchOption0")));
    QTRY_VERIFY(_window->property("flyViewActive").toBool());

    QVERIFY(openAnalyzeTools());
    QVERIFY(findVisibleItem(_rootItem, QStringLiteral("toolPanel"), TestTimeout::shortMs()));
    QVERIFY(findVisibleItem(_rootItem, QStringLiteral("analyzeButton_Onboard Logs"), TestTimeout::shortMs()));
    QVERIFY(clickButton(QStringLiteral("toolDrawerBack")));
    QTRY_VERIFY(!findVisibleItem(_rootItem, QStringLiteral("toolPanel"), 0));

    QVERIFY(openSettings());
    QQuickItem *const settingsList = findVisibleItem(_rootItem, QStringLiteral("settingsList"));
    QVERIFY(settingsList);
    const QStringList settingsPages = _visiblePageTitles(settingsList);
    QVERIFY(settingsPages.contains(QStringLiteral("General")));
    for (const QString &page : settingsPages) {
        QVERIFY2(openSettingsPage(page), qPrintable(QStringLiteral("Settings page did not open: %1").arg(page)));
    }

    stopUI();
}
