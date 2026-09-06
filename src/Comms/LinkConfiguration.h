/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#pragma once

#include <QtCore/QSettings>
#include <QtCore/QString>

class LinkInterface;

class LinkConfiguration : public QObject
{
    Q_OBJECT
    Q_MOC_INCLUDE("LinkInterface.h")

    Q_PROPERTY(QString          name            READ name           WRITE setName           NOTIFY nameChanged)
    Q_PROPERTY(LinkInterface    *link           READ link                                   NOTIFY linkChanged)
    Q_PROPERTY(LinkType         linkType        READ type                                   CONSTANT)
    Q_PROPERTY(bool             dynamic         READ isDynamic      WRITE setDynamic        NOTIFY dynamicChanged)
    Q_PROPERTY(bool             autoConnect     READ isAutoConnect  WRITE setAutoConnect    NOTIFY autoConnectChanged)
    Q_PROPERTY(QString          settingsURL     READ settingsURL                            CONSTANT)
    Q_PROPERTY(QString          settingsTitle   READ settingsTitle                          CONSTANT)
    Q_PROPERTY(bool             highLatency     READ isHighLatency  WRITE setHighLatency    NOTIFY highLatencyChanged)
    Q_PROPERTY(QString          summary         READ summary                                NOTIFY summaryChanged)
    Q_PROPERTY(QString          lastError       READ lastError                              NOTIFY lastErrorChanged)
    Q_PROPERTY(ErrorRemedy      lastErrorRemedy READ lastErrorRemedy                        NOTIFY lastErrorChanged)

public:
    LinkConfiguration(const QString &name, QObject *parent = nullptr);
    LinkConfiguration(const LinkConfiguration *copy, QObject *parent = nullptr);
    virtual ~LinkConfiguration();

    QString name() const { return _name; }
    void setName(const QString &name);

    LinkInterface *link() const { return _link.lock().get(); }
    void setLink(const std::shared_ptr<LinkInterface> link);

    virtual QString summary() const { return QString(); }

    enum ErrorRemedy {
        RemedyRetry,
        RemedyEditAddress,
    };
    Q_ENUM(ErrorRemedy)

    QString lastError() const { return _lastError; }
    ErrorRemedy lastErrorRemedy() const { return _lastErrorRemedy; }
    void setLastError(const QString &error, ErrorRemedy remedy = RemedyRetry);

    bool isDynamic() const { return _dynamic; }

    void setDynamic(bool dynamic = true);

    bool isForwarding() const { return _forwarding; }

    void setForwarding(bool forwarding = true) { _forwarding = forwarding; };

    bool isAutoConnect() const { return _autoConnect; }

    virtual void setAutoConnect(bool autoc = true);

    bool isHighLatency() const { return _highLatency; }

    void setHighLatency(bool hl = false);

    virtual void copyFrom(const LinkConfiguration *source);

    enum LinkType {
#ifndef QGC_NO_SERIAL_LINK
        TypeSerial,
#endif
        TypeUdp,
        TypeTcp,
#ifdef QGC_ENABLE_BLUETOOTH
        TypeBluetooth,
#endif
#ifdef QT_DEBUG
        TypeMock,
#endif
#ifndef QGC_AIRLINK_DISABLED
        AirLink,
#endif
        TypeLogReplay,
        TypeLast
    };
    Q_ENUM(LinkType)

    virtual LinkType type() const = 0;

    virtual void loadSettings(QSettings &settings, const QString &root) = 0;

    virtual void saveSettings(QSettings &settings, const QString &root) const = 0;

    virtual QString settingsURL() const = 0;

    virtual QString settingsTitle() const = 0;

    static LinkConfiguration *createSettings(int type, const QString &name);

    static LinkConfiguration *duplicateSettings(const LinkConfiguration *source);

    static QString settingsRoot() { return QStringLiteral("LinkConfigurations"); }

signals:
    void nameChanged(const QString &name);
    void linkChanged();
    void dynamicChanged();
    void autoConnectChanged();
    void highLatencyChanged();
    void lastErrorChanged();
    void summaryChanged();

protected:
    std::weak_ptr<LinkInterface> _link;

private:
    QString _name;
    QString _lastError;
    ErrorRemedy _lastErrorRemedy = RemedyRetry;
    bool _dynamic = false;
    bool _forwarding = false;
    bool _autoConnect = false;
    bool _highLatency = false;
};

typedef std::shared_ptr<LinkConfiguration> SharedLinkConfigurationPtr;
typedef std::weak_ptr<LinkConfiguration> WeakLinkConfigurationPtr;
