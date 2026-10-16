import { Map as MapView, Marker, AttributionControl } from 'https://cdn.jsdelivr.net/npm/maplibre-gl@6.14.0/dist/maplibre-gl.mjs';

const IMAGERY = 'https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}';
const TRAIL_POINTS = 600;
const FOLLOW_ZOOM = 17;
const EMPTY_TRAIL = { type: 'Feature', geometry: { type: 'LineString', coordinates: [] }, properties: {} };

const element = (className, html) => Object.assign(document.createElement('div'), { className, innerHTML: html });

export const createMap = (container, onFollowChange) => {
  const map = new MapView({
    container,
    style: {
      version: 8,
      sources: {
        imagery: { type: 'raster', tiles: [IMAGERY], tileSize: 256, maxzoom: 19, attribution: 'Imagery © Esri, Maxar, Earthstar Geographics' },
        trail: { type: 'geojson', data: EMPTY_TRAIL },
      },
      layers: [
        { id: 'imagery', type: 'raster', source: 'imagery' },
        { id: 'trail', type: 'line', source: 'trail', paint: { 'line-color': '#40CC6D', 'line-width': 3, 'line-opacity': 0.85 } },
      ],
    },
    center: [0, 20],
    zoom: 1.5,
    attributionControl: false,
    dragRotate: false,
  });
  map.addControl(new AttributionControl({ compact: true }), 'bottom-right');
  const vehicle = new Marker({ element: element('vehicle-marker', '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 2 20 21 12 16.5 4 21Z"/></svg>'), rotationAlignment: 'map' });
  const home = new Marker({ element: element('home-marker', 'H') });
  const follow = { on: true, placed: false, trail: [] };
  map.on('dragstart', () => {
    follow.on = false;
    onFollowChange(false);
  });

  const recenter = () => {
    follow.on = true;
    onFollowChange(true);
    const at = vehicle.getLngLat();
    if (follow.placed && at) map.easeTo({ center: at, zoom: Math.max(map.getZoom(), FOLLOW_ZOOM) });
  };

  const show = (state) => {
    const coordinate = state?.coordinate;
    if (!coordinate || !Number.isFinite(coordinate.latitude) || (coordinate.latitude === 0 && coordinate.longitude === 0)) return;
    const at = [coordinate.longitude, coordinate.latitude];
    vehicle.setLngLat(at).setRotation(state.attitude?.heading ?? 0);
    if (!follow.placed) {
      vehicle.addTo(map);
      map.jumpTo({ center: at, zoom: FOLLOW_ZOOM });
      follow.placed = true;
    } else if (follow.on) {
      map.easeTo({ center: at, duration: 250, easing: (t) => t });
    }
    const last = follow.trail.at(-1);
    if (state.armed && (!last || last[0] !== at[0] || last[1] !== at[1])) {
      follow.trail = [...follow.trail, at].slice(-TRAIL_POINTS);
      map.getSource('trail')?.setData({ ...EMPTY_TRAIL, geometry: { type: 'LineString', coordinates: follow.trail } });
    }
    const launch = state.home;
    if (launch && Number.isFinite(launch.latitude) && !(launch.latitude === 0 && launch.longitude === 0)) {
      home.setLngLat([launch.longitude, launch.latitude]).addTo(map);
    }
  };

  return { show, recenter };
};
