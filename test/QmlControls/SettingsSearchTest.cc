#include "SettingsSearchTest.h"
#include "QuickInteractionTestHelpers.h"

namespace {

QStringList callStringList(QQuickItem *root, const char *method, const QVariant &argument = QVariant())
{
    QVariant returned;
    if (argument.isValid()) {
        QMetaObject::invokeMethod(root, method, Q_RETURN_ARG(QVariant, returned), Q_ARG(QVariant, argument));
    } else {
        QMetaObject::invokeMethod(root, method, Q_RETURN_ARG(QVariant, returned));
    }
    return returned.toStringList();
}

QStringList matchedNames(QQuickItem *root, const QString &filter)
{
    return callStringList(root, "matchedNames", filter);
}

QStringList visiblePageNames(QQuickItem *root)
{
    return callStringList(root, "visiblePageNames");
}

bool matchesSyntheticPage(QQuickItem *root, bool visible, const QString &name, const QString &filter)
{
    QVariant returned;
    QMetaObject::invokeMethod(root, "matchesSyntheticPage", Q_RETURN_ARG(QVariant, returned),
                              Q_ARG(QVariant, visible), Q_ARG(QVariant, name), Q_ARG(QVariant, filter));
    return returned.toBool();
}

bool load(QQuickView &view)
{
    return loadTestView(view, QStringLiteral("qrc:/unittest/SettingsSearchTest.qml"));
}

}

void SettingsSearchTest::_emptyFilterMatchesEveryVisiblePage()
{
    QQuickView view;
    QVERIFY(load(view));

    QCOMPARE(matchedNames(view.rootObject(), QString()), visiblePageNames(view.rootObject()));
}

void SettingsSearchTest::_settingNamesInsidePagesAreFound()
{
    QQuickView view;
    QVERIFY(load(view));

    QVERIFY(matchedNames(view.rootObject(), QStringLiteral("language")).contains(QStringLiteral("General")));
    QVERIFY(matchedNames(view.rootObject(), QStringLiteral("joystick")).contains(QStringLiteral("Fly View")));
    QVERIFY(matchedNames(view.rootObject(), QStringLiteral("tile")).contains(QStringLiteral("Maps")));
    QVERIFY(matchedNames(view.rootObject(), QStringLiteral("signing")).contains(QStringLiteral("Telemetry")));
}

void SettingsSearchTest::_unknownTermMatchesNothing()
{
    QQuickView view;
    QVERIFY(load(view));

    QCOMPARE(matchedNames(view.rootObject(), QStringLiteral("propeller")).count(), 0);
}

void SettingsSearchTest::_filterCasingAndPaddingAreIgnored()
{
    QQuickView view;
    QVERIFY(load(view));

    const QStringList plain = matchedNames(view.rootObject(), QStringLiteral("maps"));
    QVERIFY(!plain.isEmpty());
    QCOMPARE(matchedNames(view.rootObject(), QStringLiteral("MAPS")), plain);
    QCOMPARE(matchedNames(view.rootObject(), QStringLiteral("  Maps  ")), plain);
}

void SettingsSearchTest::_anInvisiblePageNeverMatches()
{
    QQuickView view;
    QVERIFY(load(view));

    QVERIFY(matchesSyntheticPage(view.rootObject(), true, QStringLiteral("Widget"), QStringLiteral("widget")));
    QVERIFY(!matchesSyntheticPage(view.rootObject(), false, QStringLiteral("Widget"), QStringLiteral("widget")));
    QVERIFY(!matchesSyntheticPage(view.rootObject(), false, QStringLiteral("Widget"), QString()));
}

void SettingsSearchTest::_everyMatchedPageIsVisible()
{
    QQuickView view;
    QVERIFY(load(view));

    const QStringList visible = visiblePageNames(view.rootObject());
    const QStringList filters = { QString(), QStringLiteral("e"), QStringLiteral("mock"),
                                  QStringLiteral("debug"), QStringLiteral("palette"), QStringLiteral("settings") };
    for (const QString &filter : filters) {
        const QStringList matched = matchedNames(view.rootObject(), filter);
        for (const QString &name : matched) {
            QVERIFY2(visible.contains(name),
                     qPrintable(QStringLiteral("filter '%1' matched hidden page '%2'").arg(filter, name)));
        }
    }
}
