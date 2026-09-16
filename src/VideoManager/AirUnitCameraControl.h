#pragma once

#include <QtCore/QLoggingCategory>
#include <QtCore/QObject>
#include <QtCore/QPointer>
#include <QtCore/QString>
#include <QtCore/QTimer>

#include "MAVLinkLib.h"

Q_DECLARE_LOGGING_CATEGORY(AirUnitCameraControlLog)

class LinkInterface;

class AirUnitCameraControl : public QObject
{
    Q_OBJECT
    Q_PROPERTY(bool available   READ available   NOTIFY availableChanged)
    Q_PROPERTY(int  activeInput READ activeInput NOTIFY activeInputChanged)
    Q_PROPERTY(int  inputCount  READ inputCount  CONSTANT)
    Q_PROPERTY(QString activeInputName READ activeInputName NOTIFY activeInputChanged)
    Q_PROPERTY(QString notice READ notice NOTIFY noticeChanged)

public:
    explicit AirUnitCameraControl(QObject *parent = nullptr);

    bool available() const { return _link && !_link.isNull(); }
    int activeInput() const { return _activeInput; }
    int inputCount() const { return kInputCount; }
    QString activeInputName() const { return inputName(_activeInput); }
    QString notice() const { return _notice; }

    Q_INVOKABLE static QString inputName(int input);

    Q_INVOKABLE void selectInput(int input);
    Q_INVOKABLE void switchInput();

    void handleMessage(LinkInterface *link, const mavlink_message_t &message);

    static bool isCameraComponent(int componentId);
    static int cameraIdFromLegacyStreamInformation(const mavlink_message_t &message);

    static constexpr int kInputCount = 2;

signals:
    void availableChanged();
    void activeInputChanged();
    void noticeChanged();

protected:
    virtual void _sendMessage(LinkInterface *link, const mavlink_message_t &message);

private:
    void _sendCommand(uint16_t command, float param1);
    void _requestStreamInformation();
    void _setActiveInput(int input);
    void _showNotice(const QString &text, int milliseconds = 4000);

    QPointer<LinkInterface> _link;
    int _systemId = 0;
    int _componentId = 0;
    int _activeInput = -1;
    int _previousInput = -1;
    QString _notice;
    QTimer _noticeTimer;
};
