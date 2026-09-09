#pragma once

#ifdef QGC_RUST_CORE

#include "LinkConfiguration.h"
#include "LinkInterface.h"

#include <QtCore/QHash>
#include <QtCore/QMutex>
#include <QtCore/QString>

#include <cstdint>

class CoreLink : public LinkInterface
{
    Q_OBJECT

public:
    explicit CoreLink(SharedLinkConfigurationPtr &config, QObject *parent = nullptr);
    ~CoreLink() override;

    void disconnect() override;
    bool isConnected() const override { return _id != 0; }
    bool isSecureConnection() const override;

    uint32_t coreId() const { return _id; }

    static bool enabled();
    static bool handles(LinkConfiguration::LinkType type);
    static QByteArray configJson(const LinkConfiguration *config);

private slots:
    void _writeBytes(const QByteArray &bytes) override;

private:
    bool _connect() override;
    static void _installSinks();
    static void _bytesArrived(uint32_t id, const uint8_t *bytes, size_t len, void *user);
    static void _stateChanged(uint32_t id, bool open, const char *reason, void *user);

    uint32_t _id = 0;

    static QMutex _registryMutex;
    static QHash<uint32_t, CoreLink *> _links;
    static bool _sinksInstalled;
};

#endif
