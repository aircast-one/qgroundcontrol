#include "AppSettingsLayoutTest.h"
#include "QuickInteractionTestHelpers.h"
#include "ScreenToolsController.h"

#include <QtQml/QQmlContext>
#include <QtQml/QQmlPropertyMap>

namespace {

constexpr qreal kTolerance = 0.5;

class FakeMobileScope
{
public:
    explicit FakeMobileScope(bool mobile) { ScreenToolsController::setFakeMobile(mobile); }
    ~FakeMobileScope() { ScreenToolsController::setFakeMobile(false); }
};

struct Shell {
    QQuickItem *root     = nullptr;
    QQuickItem *settings = nullptr;
    QQuickItem *list     = nullptr;
    QQuickItem *page     = nullptr;
    QQuickItem *search   = nullptr;

    bool pageOpen() const { return settings->property("_pageOpen").toBool(); }
    bool popPage() const
    {
        bool popped = false;
        QMetaObject::invokeMethod(settings, "popPage", Q_RETURN_ARG(bool, popped));
        return popped;
    }
    void openPage(const QString &name) const
    {
        QMetaObject::invokeMethod(settings, "showSettingsPage", Q_ARG(QVariant, name));
    }
};

bool load(QQuickView &view, QQmlPropertyMap &globals, Shell &shell, int width, int height)
{
    globals.insert(QStringLiteral("commingFromRIDIndicator"), false);
    globals.insert(QStringLiteral("validationErrorCount"), 0);
    view.engine()->rootContext()->setContextProperty(QStringLiteral("globals"), &globals);
    view.resize(width, height);
    if (!loadTestView(view, QStringLiteral("qrc:/unittest/AppSettingsLayoutTest.qml"))) {
        return false;
    }
    view.rootObject()->setSize(QSizeF(width, height));
    shell.root     = view.rootObject();
    shell.settings = findItemByName(shell.root, QStringLiteral("settings"));
    shell.list     = findItemByName(shell.root, QStringLiteral("settingsList"));
    shell.page     = findItemByName(shell.root, QStringLiteral("settingsPageLoader"));
    shell.search   = findItemByName(shell.root, QStringLiteral("settingsSearchField"));
    return shell.settings && shell.list && shell.page && shell.search &&
           QTest::qWaitFor([&shell] { return shell.list->width() > 0; });
}

}

void AppSettingsLayoutTest::_phoneUprightStacksTheListAndThePage()
{
    FakeMobileScope mobile(true);
    QQmlPropertyMap globals;
    QQuickView view;
    Shell shell;
    QVERIFY(load(view, globals, shell, 428, 903));

    QVERIFY(!shell.pageOpen());
    QVERIFY(shell.list->isVisible());
    QVERIFY(shell.search->isVisible());
    QVERIFY(!shell.page->isVisible());
    QVERIFY(qAbs(shell.list->width() - (428 - shell.list->x() * 2)) <= kTolerance);
    QVERIFY(shell.settings->property("preferredWidth").toReal() == 0);

    shell.openPage(QStringLiteral("Fly View"));
    QTRY_VERIFY(shell.pageOpen());
    QVERIFY(shell.page->isVisible());
    QVERIFY(!shell.list->isVisible());
    QVERIFY(!shell.search->isVisible());
    QVERIFY(shell.page->x() + shell.page->width() > 428 * 0.9);
    QCOMPARE(shell.settings->property("pageTitle").toString(), QStringLiteral("Fly View"));

    QVERIFY(shell.popPage());
    QVERIFY(!shell.pageOpen());
    QVERIFY(shell.list->isVisible());
    QVERIFY(!shell.page->isVisible());
    QCOMPARE(shell.settings->property("pageTitle").toString(), QStringLiteral("Settings"));
    QVERIFY(!shell.popPage());
}

void AppSettingsLayoutTest::_phoneSidewaysKeepsBothColumns()
{
    FakeMobileScope mobile(true);
    QQmlPropertyMap globals;
    QQuickView view;
    Shell shell;
    QVERIFY(load(view, globals, shell, 903, 428));

    QVERIFY(shell.list->isVisible());
    QVERIFY(shell.page->isVisible());
    QVERIFY(shell.page->x() > shell.list->x() + shell.list->width());
    QVERIFY(shell.settings->property("preferredWidth").toReal() == 0);
    QVERIFY(!shell.popPage());
}

void AppSettingsLayoutTest::_desktopKeepsBothColumnsAndFloats()
{
    FakeMobileScope desktop(false);
    QQmlPropertyMap globals;
    QQuickView view;
    Shell shell;
    QVERIFY(load(view, globals, shell, 1200, 800));

    QVERIFY(shell.list->isVisible());
    QVERIFY(shell.page->isVisible());
    QVERIFY(!shell.search->isVisible());
    QVERIFY(shell.settings->property("preferredWidth").toReal() > 0);
    QVERIFY(!shell.popPage());
}
