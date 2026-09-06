#include "QGCLinksC.h"

#include "LinkConfiguration.h"
#include "LinkInterface.h"
#include "LinkManager.h"
#include "QmlObjectListModel.h"
#include "SerialLink.h"
#include "TCPLink.h"
#include "UDPLink.h"

#include <QtCore/QString>

namespace
{

LinkConfiguration *configurationAt(int index)
{
    QmlObjectListModel *const model = LinkManager::instance()->linkConfigurations();
    if (!model || (index < 0) || (index >= model->count())) {
        return nullptr;
    }
    return qobject_cast<LinkConfiguration *>(model->get(index));
}

} // namespace

int qgc_links_connect(int index)
{
    LinkConfiguration *const config = configurationAt(index);
    if (!config) {
        return 0;
    }
    LinkManager::instance()->createConnectedLink(config);
    return 1;
}

int qgc_links_disconnect(int index)
{
    LinkConfiguration *const config = configurationAt(index);
    if (!config || !config->link()) {
        return 0;
    }
    config->link()->disconnect();
    return 1;
}

int qgc_links_remove(int index)
{
    LinkConfiguration *const config = configurationAt(index);
    if (!config) {
        return 0;
    }
    LinkManager::instance()->removeConfiguration(config);
    return 1;
}

int qgc_links_create(int type, const char *name, const char *host, int port)
{
    const QString linkName = QString::fromUtf8(name).trimmed();
    if (linkName.isEmpty()) {
        return 0;
    }

    LinkConfiguration *const config = LinkManager::instance()->createConfiguration(type, linkName);
    if (!config) {
        return 0;
    }

    // createConfiguration hands back an unowned object; endCreateConfiguration is what
    // adopts it into the model and persists it.
    if (TCPConfiguration *const tcp = qobject_cast<TCPConfiguration *>(config)) {
        tcp->setHost(QString::fromUtf8(host));
        tcp->setPort(static_cast<quint16>(port));
    } else if (UDPConfiguration *const udp = qobject_cast<UDPConfiguration *>(config)) {
        udp->setLocalPort(static_cast<quint16>(port));
#ifndef QGC_NO_SERIAL_LINK
    } else if (SerialConfiguration *const serial = qobject_cast<SerialConfiguration *>(config)) {
        // host carries the device path and port the baud rate for a serial link.
        serial->setPortName(QString::fromUtf8(host));
        if (port > 0) {
            serial->setBaud(port);
        }
#endif
    }

    LinkManager::instance()->endCreateConfiguration(config);
    return 1;
}
