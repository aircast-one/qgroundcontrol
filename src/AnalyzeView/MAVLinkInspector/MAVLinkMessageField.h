#pragma once

#include <QtCore/QObject>
#include <QtCore/QPointF>
#include <QtCore/QString>
#include <QtQmlIntegration/QtQmlIntegration>

#include <limits>

class QGCMAVLinkMessage;
class MAVLinkChartController;
class QAbstractSeries;

class QGCMAVLinkMessageField : public QObject
{
    Q_OBJECT
    // QML_ELEMENT
#ifndef QGC_HEADLESS_CORE
    Q_MOC_INCLUDE(<QtGraphs/QAbstractSeries>)
#endif
    Q_PROPERTY(QString                  name        READ name       CONSTANT)
    Q_PROPERTY(QString                  label       READ label      CONSTANT)
    Q_PROPERTY(QString                  type        READ type       CONSTANT)
    Q_PROPERTY(QString                  value       READ value      NOTIFY valueChanged)
    Q_PROPERTY(bool                     selectable  READ selectable NOTIFY selectableChanged)
    Q_PROPERTY(int                      chartIndex  READ chartIndex NOTIFY seriesChanged)
#ifndef QGC_HEADLESS_CORE
    Q_PROPERTY(const QAbstractSeries    *series     READ series     NOTIFY seriesChanged)
#endif

public:
    QGCMAVLinkMessageField(const QString &name, const QString &type, QGCMAVLinkMessage *parent = nullptr);
    ~QGCMAVLinkMessageField();

    QString name() const { return _name;  }
    QString label() const;
    QString type() const { return _type;  }
    QString value() const { return _value; }
    bool selectable() const { return _selectable; }
    bool selected() const { return !!_pSeries; }
#ifndef QGC_HEADLESS_CORE
    const QAbstractSeries *series() const { return _pSeries; }
#endif
    const QList<QPointF> *values() const { return &_values; }
    qreal rangeMin() const { return _rangeMin; }
    qreal rangeMax() const { return _rangeMax; }
    int chartIndex() const;

    void setSelectable(bool sel);
    void updateValue(const QString &newValue, qreal v);
    void resetBucketing(int bucketCount, qreal bucketWidthMs);

#ifndef QGC_HEADLESS_CORE
    void addSeries(MAVLinkChartController *chartController, QAbstractSeries *series);
#endif
    void delSeries();
    void updateSeries();

signals:
    void seriesChanged();
    void selectableChanged();
    void valueChanged();

private:
    void _commitBucket();

    QString _type;
    QString _name;
    QGCMAVLinkMessage *_msg = nullptr;

    QString _value;
    bool _selectable = true;
    int _dataIndex = 0;
    int _bucketCount = 0;
    qreal _bucketWidthMs = 0;
    qreal _currentBucketStart = -1;
    qreal _currentBucketMin = 0;
    qreal _currentBucketMax = 0;
    qreal _rangeMin = std::numeric_limits<qreal>::max();
    qreal _rangeMax = std::numeric_limits<qreal>::lowest();
    QList<QPointF> _values;

    QAbstractSeries *_pSeries = nullptr;
    MAVLinkChartController *_chartController = nullptr;
};
