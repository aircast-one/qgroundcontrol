/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#pragma once

#include "UnitTest.h"

class QGCBridgeCoreTest : public UnitTest
{
    Q_OBJECT

private slots:
    void init();
    void cleanup();

    void _readsScalarProperty();
    void _readsFactWithMetadata();
    void _readsFactsOfSettingsGroup();
    void _writesFactValue();
    void _writesEnumIndex();
    void _resolvesListIndex();
    void _invokesFactValidateAndReturnsQGCsOwnError();
    void _factCarriesWhatQGCKnowsAboutItsRange();
    void _watchFromAnotherThreadDoesNotBlockTheCaller();
    void _writesAnObjectPropertyFromAnAtPath();
    void _planStartsWithItsSettingsItemAndNoVehicle();
    void _planSaveReportsWhetherItWrote();
    void _aRefusedLoadLeavesTheExistingPlanAlone();
    void _resolvesAccessorCall();
    void _writesThroughAccessorCall();
    void _invokeReturnsValue();
    void _invokeConvertsArguments();
    void _rejectsUnknownPaths();
    void _resolvesLogDownloadRoot();
    void _resolvesMavlinkConsoleRoot();
    void _resolvesMavlinkInspectorRoot();
    void _invokeReturnsAFactObject();
    void _invokeRejectsUnresolvableObjectReference();
    void _invokeRejectsTooManyArguments();
    void _accessorCallNeedsAQObjectReturn();
    void _vehicleRootIsNullWithoutAnActiveVehicle();
    void _sensorsCalRootIsNullWithoutAnApmVehicle();
    void _radioCalRootIsNullWithoutAnActiveVehicle();
    void _setRejectsAPayloadWithoutAValue();
    void _writeSaysWhyItFailed();
    void _indexesAVariantListOfObjects();
    void _watchEmitsOnChange();
    void _watchReplacesItsPreviousPaths();
    void _watchStopsWhenGivenNoPaths();
};
