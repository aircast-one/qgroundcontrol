const TERRARIUM = "https://s3.amazonaws.com/elevation-tiles-prod/terrarium";
const TERRARIUM_LEVEL = 15;
const TILE_PX = 256;
const SAMPLES = 65;
const KEPT_TILES = 64;
const HOME_SAMPLE_LEVEL = 14;
const CLEARANCE_M = 2;
const FOLLOW = 0.25;
const FRAME_RATE = 30;

const decoded = new Map();

const remember = (key, tile) => {
  decoded.delete(key);
  decoded.set(key, tile);
  [...decoded.keys()].slice(0, Math.max(0, decoded.size - KEPT_TILES)).forEach((old) => decoded.delete(old));
  return tile;
};

const fetchTerrarium = (key) =>
  fetch(`${TERRARIUM}/${key}.png`)
    .then((response) => (response.ok ? response.blob() : Promise.reject(new Error(`terrain ${key}: ${response.status}`))))
    .then((blob) => createImageBitmap(blob, { colorSpaceConversion: "none", premultiplyAlpha: "none" }))
    .then((bitmap) => {
      const context = new OffscreenCanvas(TILE_PX, TILE_PX).getContext("2d", { willReadFrequently: true });
      context.drawImage(bitmap, 0, 0);
      return context.getImageData(0, 0, TILE_PX, TILE_PX).data;
    })
    .catch((error) => {
      decoded.delete(key);
      throw error;
    });

const terrariumTile = (level, x, y) => {
  const key = `${level}/${x}/${y}`;
  return remember(key, decoded.get(key) ?? fetchTerrarium(key));
};

const elevation = (pixels, column, row) => {
  const at = (row * TILE_PX + column) * 4;
  return pixels[at] * 256 + pixels[at + 1] + pixels[at + 2] / 256 - 32768;
};

const sampled = (pixels, u, v) => {
  const [column, row] = [Math.min(Math.max(u - 0.5, 0), TILE_PX - 1), Math.min(Math.max(v - 0.5, 0), TILE_PX - 1)];
  const [left, top] = [Math.floor(column), Math.floor(row)];
  const [right, bottom] = [Math.min(left + 1, TILE_PX - 1), Math.min(top + 1, TILE_PX - 1)];
  const [across, down] = [column - left, row - top];
  const upper = elevation(pixels, left, top) * (1 - across) + elevation(pixels, right, top) * across;
  const lower = elevation(pixels, left, bottom) * (1 - across) + elevation(pixels, right, bottom) * across;
  return upper * (1 - down) + lower * down;
};

const heights = (x, y, level) => {
  const source = Math.min(level, TERRARIUM_LEVEL);
  const shift = level - source;
  const span = TILE_PX / 2 ** shift;
  const [left, top] = [(x - ((x >> shift) << shift)) * span, (y - ((y >> shift) << shift)) * span];
  return terrariumTile(source, x >> shift, y >> shift).then((pixels) =>
    Float32Array.from({ length: SAMPLES * SAMPLES }, (_, index) =>
      sampled(pixels, left + ((index % SAMPLES) / (SAMPLES - 1)) * span, top + (Math.floor(index / SAMPLES) / (SAMPLES - 1)) * span),
    ),
  );
};

const terrain = new Cesium.CustomHeightmapTerrainProvider({
  width: SAMPLES,
  height: SAMPLES,
  tilingScheme: new Cesium.WebMercatorTilingScheme(),
  callback: heights,
  credit: "Terrain: Mapzen, AWS Open Data",
});

const widget = new Cesium.CesiumWidget("view", {
  baseLayer: false,
  terrainProvider: terrain,
  skyBox: false,
  scene3DOnly: true,
  creditContainer: "credits",
  targetFrameRate: FRAME_RATE,
});
const scene = widget.scene;
scene.screenSpaceCameraController.enableInputs = false;
scene.globe.showGroundAtmosphere = true;
scene.backgroundColor = Cesium.Color.BLACK;

const state = { target: undefined, shown: undefined, imagery: undefined, home: undefined, homeGround: undefined };

const showImagery = (name) => {
  state.imagery = name;
  scene.imageryLayers.removeAll();
  scene.imageryLayers.addImageryProvider(
    new Cesium.UrlTemplateImageryProvider({ url: `https://qgc.tiles/${encodeURIComponent(name)}/{z}/{x}/{y}`, maximumLevel: 19, credit: name }),
  );
};

const anchorHome = (pose) => {
  const key = `${pose.homeLatitude},${pose.homeLongitude}`;
  state.home = key;
  state.homeGround = undefined;
  Cesium.sampleTerrain(terrain, HOME_SAMPLE_LEVEL, [Cesium.Cartographic.fromDegrees(pose.homeLongitude, pose.homeLatitude)])
    .then(([ground]) => {
      state.homeGround = state.home === key ? ground.height : state.homeGround;
    })
    .catch(() => {
      state.home = state.home === key ? undefined : state.home;
    });
};

const wrapped = (degrees) => ((((degrees + 180) % 360) + 360) % 360) - 180;

const toward = (from, to) => ({
  longitude: from.longitude + (to.longitude - from.longitude) * FOLLOW,
  latitude: from.latitude + (to.latitude - from.latitude) * FOLLOW,
  height: from.height + (to.height - from.height) * FOLLOW,
  heading: from.heading + wrapped(to.heading - from.heading) * FOLLOW,
  pitch: from.pitch + (to.pitch - from.pitch) * FOLLOW,
  roll: from.roll + (to.roll - from.roll) * FOLLOW,
  fov: to.fov,
});

const goal = (target) => {
  const ground = scene.globe.getHeight(Cesium.Cartographic.fromDegrees(target.longitude, target.latitude));
  const aboveHome = state.homeGround + target.aboveHome;
  return {
    longitude: target.longitude,
    latitude: target.latitude,
    height: Math.max(aboveHome, (ground ?? -Infinity) + CLEARANCE_M),
    heading: target.heading,
    pitch: target.pitch,
    roll: target.roll,
    fov: target.fov,
  };
};

const place = (pose) => {
  widget.camera.setView({
    destination: Cesium.Cartesian3.fromDegrees(pose.longitude, pose.latitude, pose.height),
    orientation: { heading: Cesium.Math.toRadians(pose.heading), pitch: Cesium.Math.toRadians(pose.pitch), roll: Cesium.Math.toRadians(pose.roll) },
  });
  widget.camera.frustum.fov = Cesium.Math.toRadians(pose.fov);
  document.body.classList.add("placed");
};

scene.preRender.addEventListener(() => {
  const next = state.target && state.homeGround !== undefined ? goal(state.target) : undefined;
  state.shown = next && (state.shown ? toward(state.shown, next) : next);
  return state.shown && place(state.shown);
});

window.aircast = {
  pose: (pose) => {
    state.target = pose.available ? pose : undefined;
    return pose.available && [
      pose.imagery !== state.imagery && showImagery(pose.imagery),
      `${pose.homeLatitude},${pose.homeLongitude}` !== state.home && anchorHome(pose),
    ];
  },
};
