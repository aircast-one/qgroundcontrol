#pragma once

#include "QmlUITestBase.h"

/// UI smoke test that loads MainWindow and navigates through the top-level views
/// (Fly, Plan, Analyze, Settings) and every always-available settings page,
/// failing on any QML error or warning raised along the way.
class TopLevelViewsTest : public QmlUITestBase
{
    Q_OBJECT

public:
    TopLevelViewsTest() = default;

private slots:
    void _testNavigateViews();
};
