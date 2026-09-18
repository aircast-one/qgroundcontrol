#pragma once

class QSettings;

namespace QGCSettingsRecovery {
bool moveAsideIfUnwritable(QSettings &settings);
}
