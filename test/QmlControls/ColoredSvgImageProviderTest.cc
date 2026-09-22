#include "ColoredSvgImageProviderTest.h"

#include <QtCore/QRegularExpression>
#include <QtTest/QTest>

#include "ColoredSvgImageProvider.h"

namespace {

QImage request(const QString &path, const QSize &requested = QSize(32, 32))
{
    ColoredSvgImageProvider provider;
    QSize actual;
    return provider.requestImage(path + QStringLiteral("?color=FF0000"), &actual, requested);
}

bool anyOpaquePixelIs(const QImage &image, QRgb want)
{
    for (int y = 0; y < image.height(); y++) {
        for (int x = 0; x < image.width(); x++) {
            const QColor pixel = image.pixelColor(x, y);
            if ((pixel.alpha() > 0) && (pixel.rgb() == want)) {
                return true;
            }
        }
    }
    return false;
}

} // namespace

void ColoredSvgImageProviderTest::_tintsAnSvg()
{
    const QImage image = request(QStringLiteral("/res/OpenDoor.svg"));

    QVERIFY(!image.isNull());
    QVERIFY(anyOpaquePixelIs(image, qRgb(255, 0, 0)));
}

/// qgcresources.qrc aliases QGCLogoWhite.png onto a .svg name. Choosing the
/// renderer by suffix handed those bytes to QSvgRenderer, which rejected them,
/// and the Settings entry in the view menu drew no icon at all.
void ColoredSvgImageProviderTest::_rasterAliasedAsSvgStillDraws()
{
    const QImage image = request(QStringLiteral("/res/QGCLogoWhite.svg"));

    QVERIFY(!image.isNull());
    QVERIFY(anyOpaquePixelIs(image, qRgb(255, 0, 0)));
}

void ColoredSvgImageProviderTest::_missingResourceDrawsNothing()
{
    expectLogMessage("QmlControls.ColoredSvgImageProvider", QtWarningMsg, QRegularExpression("QSvgRenderer rejected"));
    ignoreLogMessage("qt.svg", QtWarningMsg, QRegularExpression("Cannot open file"));

    const QImage image = request(QStringLiteral("/res/NoSuchIcon.svg"));
    verifyExpectedLogMessage();

    QVERIFY(image.isNull());
}

UT_REGISTER_TEST(ColoredSvgImageProviderTest, TestLabel::Unit)
