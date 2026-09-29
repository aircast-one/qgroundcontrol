#pragma once

#include "UnitTest.h"

class QGCNetworkHelperTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _testClassifyHttpStatusInformational();
    void _testClassifyHttpStatusSuccess();
    void _testClassifyHttpStatusRedirection();
    void _testClassifyHttpStatusClientError();
    void _testClassifyHttpStatusServerError();
    void _testClassifyHttpStatusUnknown();
    void _testIsHttpSuccess();
    void _testIsHttpRedirect();
    void _testIsHttpClientError();
    void _testIsHttpServerError();
    void _testHttpStatusText();
    void _testHttpStatusTextFromEnum();
    void _testHttpStatusCodeEnumRoundTrip();

    void _testHttpMethodName();
    void _testParseHttpMethod();
    void _testParseHttpMethodCaseInsensitive();
    void _testParseHttpMethodUnknown();

    void _testIsValidUrl();
    void _testIsHttpUrl();
    void _testIsHttpsUrl();
    void _testNormalizeUrl();
    void _testEnsureScheme();
    void _testBuildUrlFromMap();
    void _testBuildUrlFromList();
    void _testUrlWithoutQuery();

    void _testDefaultUserAgent();
    void _testRequestConfigDefaults();
    void _testRequestConfigAttributes();
    void _testRequestConfigKeepAlive();
    void _testSetJsonHeaders();
    void _testCreateBasicAuthCredentials();
    void _testSetBasicAuthHeader();
    void _testSetBearerTokenHeader();
    void _testLooksLikeJson();
    void _testErrorMessageNullReply();
    void _testRedirectUrlResolvesRelativeTarget();
    void _testErrorMessagePrefersNetworkErrorOverHttpStatus();
    void _testParseJsonValid();
    void _testParseJsonInvalid();
    void _testParseJsonReplyNull();
    void _testReplyHelpersNullReply();

    void _testIsNetworkAvailable();
    void _testIsBluetoothAvailableDoesNotBlock();
    void _testConnectionTypeName();

    void _testIsSslAvailable();
    void _testSslVersion();
};
