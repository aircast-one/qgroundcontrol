#include "QGCLinksC.h"

#include "LinkConfiguration.h"
#include "LinkManager.h"
#include "QmlObjectListModel.h"
#include "SerialLink.h"
#include "QGCQtThread.h"
#include "TCPLink.h"
#include "UDPLink.h"

#include <QtCore/QString>

int qgc_links_create(int type, const char *name, const char *host, int port)
{
    const QString linkName = QString::fromUtf8(name).trimmed();
    if (linkName.isEmpty()) {
        return 0;
    }

    const QString hostName = QString::fromUtf8(host);

    return qgcOnQtThread([&]() -> int {
        LinkConfiguration *const config = LinkManager::instance()->createConfiguration(type, linkName);
        if (!config) {
            return 0;
        }

        // createConfiguration hands back an unowned object; endCreateConfiguration is what
        // adopts it into the model and persists it.
        if (TCPConfiguration *const tcp = qobject_cast<TCPConfiguration *>(config)) {
            tcp->setHost(hostName);
            tcp->setPort(static_cast<quint16>(port));
        } else if (UDPConfiguration *const udp = qobject_cast<UDPConfiguration *>(config)) {
            udp->setLocalPort(static_cast<quint16>(port));
#ifndef QGC_NO_SERIAL_LINK
        } else if (SerialConfiguration *const serial = qobject_cast<SerialConfiguration *>(config)) {
            // host carries the device path and port the baud rate for a serial link.
            serial->setPortName(hostName);
            if (port > 0) {
                serial->setBaud(port);
            }
#endif
        }

        LinkManager::instance()->endCreateConfiguration(config);
        return 1;
    });
}
