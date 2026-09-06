#include "QGCEntry.h"

#include "QGCApplication.h"

#include <QtGui/QWindow>
#include <QtQuick/QQuickWindow>

namespace
{

QWindow *g_container = nullptr;

} // namespace

void qgc_embed_main_window(void *native_view)
{
    QQuickWindow *const root = qgcApp() ? qgcApp()->mainRootWindow() : nullptr;
    if (!root || !native_view) {
        return;
    }

    g_container = QWindow::fromWinId(reinterpret_cast<WId>(native_view));
    if (!g_container) {
        return;
    }

    root->setFlags(Qt::FramelessWindowHint);
    root->setParent(g_container);
    root->setPosition(0, 0);
    root->resize(g_container->size());
    root->show();
}

void qgc_resize_main_window(int width, int height)
{
    QQuickWindow *const root = qgcApp() ? qgcApp()->mainRootWindow() : nullptr;
    if (!root || (width <= 0) || (height <= 0)) {
        return;
    }

    root->setPosition(0, 0);
    root->resize(width, height);
}
