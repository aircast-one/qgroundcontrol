#pragma once

#include <QtCore/QCoreApplication>
#include <QtCore/QMetaObject>
#include <QtCore/QThread>

/// Runs @p body on the thread that owns the application object, blocking until it returns.
/// A host that owns the main thread calls the C ABI from there, and Qt objects may only be
/// touched on Qt's own thread.
template <typename Fn>
auto qgcOnQtThread(Fn body) -> decltype(body())
{
    QCoreApplication *const app = QCoreApplication::instance();
    if (!app || (QThread::currentThread() == app->thread())) {
        return body();
    }

    decltype(body()) result{};
    (void) QMetaObject::invokeMethod(app, [&] { result = body(); }, Qt::BlockingQueuedConnection);
    return result;
}

template <typename Fn>
void qgcOnQtThreadVoid(Fn body)
{
    QCoreApplication *const app = QCoreApplication::instance();
    if (!app || (QThread::currentThread() == app->thread())) {
        body();
        return;
    }

    (void) QMetaObject::invokeMethod(app, [&] { body(); }, Qt::BlockingQueuedConnection);
}
