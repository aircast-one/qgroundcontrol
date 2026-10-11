const test = require("node:test");
const assert = require("node:assert/strict");
const tiles = require("../../main/assets/synthetic/tiles.js");

test("a position lands in the slippy-map tile that holds it", () => {
  assert.deepEqual(tiles.tileOf(44.8271, 41.7151, 14), { x: 10232, y: 6099 });
  const corner = [tiles.longitudeAt(10240, 14), tiles.latitudeAt(6105, 14)];
  assert.deepEqual(tiles.tileOf(corner[0] + 1e-9, corner[1] - 1e-9, 14), { x: 10240, y: 6105 }, "a tile's north-west corner maps back into that tile");
});

test("buildings are wanted from every tile within reach of the aircraft", () => {
  assert.deepEqual(tiles.around(30.5234, 50.4501, 1000, 14).toSorted(), ["9580/5523", "9580/5524", "9580/5525", "9581/5523", "9581/5524", "9581/5525"]);
});

test("reach wraps across the antimeridian and stops at the edge of the map", () => {
  const dateline = tiles.around(179.999, 0, 1000, 14);
  assert.ok(dateline.some((key) => key.startsWith("16383/")) && dateline.some((key) => key.startsWith("0/")), "both sides of 180 degrees are fetched");
  assert.ok(dateline.every((key) => Number(key.split("/")[0]) >= 0 && Number(key.split("/")[0]) < 2 ** 14), "no column off the map");
  const pole = tiles.around(0, 85.05, 1000, 14);
  assert.ok(pole.every((key) => Number(key.split("/")[1]) >= 0), "no row above the top of the map");
});

test("a served ring of x,y numbers becomes corner pairs", () => {
  assert.deepEqual(tiles.corners([0, 0, 10, 0, 10, 10]), [[0, 0], [10, 0], [10, 10]]);
});

test("the oldest tiles out of reach go first, and only past the limit", () => {
  assert.deepEqual(tiles.evicted(["a", "b", "c", "d"], ["d"], 2), ["a", "b"]);
  assert.deepEqual(tiles.evicted(["a", "b"], ["a"], 9), []);
  assert.deepEqual(tiles.evicted(["a", "b", "c"], ["a", "b", "c"], 1), [], "a wanted tile is never evicted, however many are wanted");
});

test("a failed tile is retried only after the wait", () => {
  assert.equal(tiles.retryDue(undefined, 50000, 10000), false, "a tile that has not failed is not retried");
  assert.equal(tiles.retryDue(0, 5000, 10000), false);
  assert.equal(tiles.retryDue(0, 10001, 10000), true);
});
