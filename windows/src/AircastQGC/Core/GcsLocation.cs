using System;
using System.Runtime.InteropServices;
using Windows.Devices.Geolocation;

namespace AircastQGC.Core;

public static partial class GcsLocation
{
    public const string SourceToken = "internalGps";
    private const long AccessError = 0;
    private const long ClosedError = 1;
    private const uint ReportIntervalMs = 1000;

    private static Geolocator? _locator;

    public static async void Start()
    {
        if (_locator != null) return;
        try
        {
            var access = await Geolocator.RequestAccessAsync();
            if (access != GeolocationAccessStatus.Allowed)
            {
                qgc_core_gcs_position_error(AccessError);
                return;
            }
            var locator = new Geolocator { DesiredAccuracy = PositionAccuracy.High, ReportInterval = ReportIntervalMs };
            locator.PositionChanged += OnPosition;
            locator.StatusChanged += OnStatus;
            _locator = locator;
        }
        catch (UnauthorizedAccessException)
        {
            qgc_core_gcs_position_error(AccessError);
        }
        catch (Exception)
        {
            qgc_core_gcs_position_error(ClosedError);
        }
    }

    public static void Stop()
    {
        if (_locator is { } locator)
        {
            locator.PositionChanged -= OnPosition;
            locator.StatusChanged -= OnStatus;
        }
        _locator = null;
    }

    private static void OnPosition(Geolocator sender, PositionChangedEventArgs args)
    {
        var coordinate = args.Position.Coordinate;
        var position = coordinate.Point.Position;
        qgc_core_gcs_position_update(
            position.Latitude,
            position.Longitude,
            coordinate.AltitudeAccuracy.HasValue ? position.Altitude : double.NaN,
            coordinate.Accuracy >= 0 ? coordinate.Accuracy : double.NaN,
            coordinate.AltitudeAccuracy ?? double.NaN,
            coordinate.Heading ?? double.NaN,
            double.NaN);
    }

    private static void OnStatus(Geolocator sender, StatusChangedEventArgs args)
    {
        if (args.Status == PositionStatus.Disabled) qgc_core_gcs_position_error(AccessError);
        else if (args.Status == PositionStatus.NotAvailable) qgc_core_gcs_position_error(ClosedError);
    }

    [LibraryImport("groundstation")]
    private static partial void qgc_core_gcs_position_update(
        double latitude, double longitude, double altitude,
        double horizontalAccuracyM, double verticalAccuracyM,
        double directionDeg, double directionAccuracyDeg);

    [LibraryImport("groundstation")]
    private static partial void qgc_core_gcs_position_error(long code);
}
