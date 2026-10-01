#pragma once

#include <QtCore/QString>
#include <QtCore/QStringList>

class LinkManager;

namespace RadiomasterAx12 {
inline const QString kManufacturer = QStringLiteral("Radiomaster");
inline const QString kElrsSerialPort = QStringLiteral("ttyS1");
inline const QString kElrsLinkName = QStringLiteral("AX12 ELRS");
constexpr qint32 kElrsBaudRate = 460800;

bool isRemote(const QString& manufacturer);
bool defaultsToBuiltInSerial(bool isAx12, const QStringList& builtInPortNames);
bool isThisRemote();
#ifndef QGC_NO_SERIAL_LINK
bool ensureElrsLink(LinkManager* linkManager);
#endif
}  // namespace RadiomasterAx12
