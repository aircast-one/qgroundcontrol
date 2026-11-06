#pragma once

#include <QtCore/QObject>
#include <QtCore/QString>
#include <QtCore/QTimer>
#include <QtQmlIntegration/QtQmlIntegration>
#include <optional>

#include "AppUpdateOffer.h"

class QGCFileDownload;

class AppUpdater : public QObject
{
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("")

    Q_PROPERTY(QString state READ state NOTIFY stateChanged)
    Q_PROPERTY(QString version READ version NOTIFY stateChanged)
    Q_PROPERTY(QString error READ error NOTIFY stateChanged)

public:
    explicit AppUpdater(QObject* parent = nullptr);

    static AppUpdater* instance();
    static constexpr int kFirstCheckAfterMs = 15 * 1000;
    static constexpr int kCheckEveryMs = 6 * 60 * 60 * 1000;
    static constexpr int kOfferAgainAfterMs = 60 * 1000;

    void start();

    QString state() const { return _state; }

    QString version() const { return _offer ? _offer->version : QString(); }

    QString error() const { return _error; }

    QString file() const { return _file; }

    Q_INVOKABLE void check();
    Q_INVOKABLE bool install();
    Q_INVOKABLE void later();

signals:
    void stateChanged();
    void readyToInstall(const QString& text);

private:
    static bool _vehicleConnected();
    QString _updatesDir() const;
    void _setState(const QString& state, const QString& error = QString());
    void _tick();
    void _onCheckFinished(bool success, const QString& localFile, const QString& errorMsg);
    void _onListingFinished(bool success, const QString& localFile, const QString& errorMsg);
    void _onDownloadFinished(bool success, const QString& localFile, const QString& errorMsg);
    void _offerIfFree();
    QGCFileDownload* _fetch(const QString& url, void (AppUpdater::*done)(bool, const QString&, const QString&));

    QTimer _timer;
    QTimer _offerTimer;
    QString _state = QStringLiteral("idle");
    QString _error;
    QString _file;
    QString _sha256;
    QString _askedFor;
    bool _checking = false;
    std::optional<AppUpdateOffer::Offer> _offer;
};
