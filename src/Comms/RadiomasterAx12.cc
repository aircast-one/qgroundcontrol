#include "RadiomasterAx12.h"

#ifndef QGC_NO_SERIAL_LINK
#include "LinkManager.h"
#include "QmlObjectListModel.h"
#include "SerialLink.h"
#endif

#ifdef Q_OS_ANDROID
#include <QtCore/QJniObject>
#endif

bool RadiomasterAx12::isRemote(const QString& manufacturer)
{
    return manufacturer.compare(kManufacturer, Qt::CaseInsensitive) == 0;
}

bool RadiomasterAx12::defaultsToBuiltInSerial(bool isAx12, const QStringList& builtInPortNames)
{
    return isAx12 && builtInPortNames.contains(kElrsSerialPort);
}

bool RadiomasterAx12::isThisRemote()
{
#ifdef Q_OS_ANDROID
    return isRemote(QJniObject::getStaticObjectField<jstring>("android/os/Build", "MANUFACTURER").toString());
#else
    return false;
#endif
}

#ifndef QGC_NO_SERIAL_LINK
namespace {

SerialConfiguration* findElrsConfig(LinkManager* linkManager)
{
    QmlObjectListModel* configs = linkManager->linkConfigurations();
    for (int i = 0; i < configs->count(); i++) {
        auto* config = qobject_cast<LinkConfiguration*>(configs->get(i));
        if (config && config->type() == LinkConfiguration::TypeSerial) {
            auto* serial = static_cast<SerialConfiguration*>(config);
            if (serial->portName().section(QLatin1Char('/'), -1) == RadiomasterAx12::kElrsSerialPort) {
                return serial;
            }
        }
    }
    return nullptr;
}

}  // namespace

bool RadiomasterAx12::ensureElrsLink(LinkManager* linkManager)
{
    SerialConfiguration* existing = findElrsConfig(linkManager);
    if (existing && existing->isAutoConnect()) {
        return false;
    }
    if (existing) {
        existing->setAutoConnect(true);
    } else {
        auto* config = new SerialConfiguration(kElrsLinkName);
        config->setPortName(QStringLiteral("/dev/") + kElrsSerialPort);
        config->setBaud(kElrsBaudRate);
        config->setAutoConnect(true);
        (void) linkManager->addConfiguration(config);
    }
    linkManager->saveLinkConfigurationList();
    return true;
}
#endif
