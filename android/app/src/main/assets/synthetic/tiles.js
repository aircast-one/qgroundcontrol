const SyntheticTiles = (() => {
  const EARTH_RADIUS_M = 6371000;
  const radians = (degrees) => (degrees * Math.PI) / 180;
  const degrees = (radians) => (radians * 180) / Math.PI;
  const span = (level) => 2 ** level;
  const wrapped = (x, level) => ((x % span(level)) + span(level)) % span(level);
  const clamped = (y, level) => Math.min(Math.max(y, 0), span(level) - 1);

  const tileOf = (longitude, latitude, level) => {
    const rad = radians(latitude);
    return {
      x: Math.floor(((longitude + 180) / 360) * span(level)),
      y: Math.floor(((1 - Math.log(Math.tan(rad) + 1 / Math.cos(rad)) / Math.PI) / 2) * span(level)),
    };
  };

  const longitudeAt = (x, level) => (x / span(level)) * 360 - 180;

  const latitudeAt = (y, level) => degrees(Math.atan(Math.sinh(Math.PI * (1 - (2 * y) / span(level)))));

  const around = (longitude, latitude, reach, level) => {
    const across = degrees(reach / EARTH_RADIUS_M);
    const along = across / Math.cos(radians(latitude));
    const [from, to] = [tileOf(longitude - along, latitude + across, level), tileOf(longitude + along, latitude - across, level)];
    const columns = Array.from({ length: to.x - from.x + 1 }, (_, column) => wrapped(from.x + column, level));
    const rows = Array.from({ length: to.y - from.y + 1 }, (_, row) => clamped(from.y + row, level));
    return [...new Set(columns.flatMap((x) => rows.map((y) => `${x}/${y}`)))];
  };

  const corners = (ring) => Array.from({ length: ring.length / 2 }, (_, at) => [ring[2 * at], ring[2 * at + 1]]);

  const evicted = (held, wanted, kept) => held.filter((key) => !wanted.includes(key)).slice(0, Math.max(0, held.length - kept));

  const retryDue = (failedAt, now, wait) => failedAt !== undefined && now - failedAt > wait;

  return { tileOf, longitudeAt, latitudeAt, around, corners, evicted, retryDue };
})();

typeof module !== "undefined" && (module.exports = SyntheticTiles);
