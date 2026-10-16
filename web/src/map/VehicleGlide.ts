import { coerceIn, same } from '@/kotlin/Std';
import { isPlottable } from '@/map/VehicleMap';
import type { VehicleChoice } from '@/map/VehiclePicker';

const SHORTEST_GLIDE_NS = 100_000_000;
const LONGEST_GLIDE_NS = 1_000_000_000;

export type VehiclePose = { latitude: number; longitude: number; heading: number };

export const VehiclePose = (fields: VehiclePose): VehiclePose => ({ ...fields });

export const UNPLACED: VehiclePose = VehiclePose({ latitude: NaN, longitude: NaN, heading: NaN });

export class Glide {
  constructor(
    readonly from: VehiclePose,
    readonly to: VehiclePose,
    readonly startNs: number,
    readonly endNs: number,
    readonly fixedAtNs: number,
  ) {}

  static settled(pose: VehiclePose, nowNs: number): Glide {
    return new Glide(pose, pose, nowNs, nowNs, nowNs);
  }

  at(nowNs: number): VehiclePose {
    const { from, to, startNs, endNs } = this;
    const fraction = endNs <= startNs ? 1.0 : coerceIn((nowNs - startNs) / (endNs - startNs), 0.0, 1.0);
    const turn = ((((to.heading - from.heading) % 360) + 540) % 360) - 180;
    return VehiclePose({
      latitude: from.latitude + (to.latitude - from.latitude) * fraction,
      longitude: from.longitude + (to.longitude - from.longitude) * fraction,
      heading: Number.isNaN(from.heading) ? to.heading : from.heading + turn * fraction,
    });
  }

  toward(target: VehiclePose, nowNs: number): Glide {
    const shown = this.at(nowNs);
    const moved = target.latitude !== this.to.latitude || target.longitude !== this.to.longitude;
    if (same(target, this.to)) return this;
    if (!isPlottable(shown.latitude, shown.longitude) || !isPlottable(target.latitude, target.longitude)) return Glide.settled(target, nowNs);
    if (moved) return new Glide(shown, target, nowNs, nowNs + coerceIn(nowNs - this.fixedAtNs, SHORTEST_GLIDE_NS, LONGEST_GLIDE_NS), nowNs);
    return new Glide(shown, target, nowNs, Math.max(this.endNs, nowNs + SHORTEST_GLIDE_NS), this.fixedAtNs);
  }
}

export const toward = (glides: ReadonlyMap<number, Glide>, fleet: readonly VehicleChoice[], nowNs: number): ReadonlyMap<number, Glide> =>
  new Map(fleet.map((it) => [it.id, (glides.get(it.id) ?? Glide.settled(UNPLACED, 0)).toward(VehiclePose({ latitude: it.latitude, longitude: it.longitude, heading: it.heading }), nowNs)]));

export const glidedAt = (fleet: readonly VehicleChoice[], glides: ReadonlyMap<number, Glide>, nowNs: number): VehicleChoice[] =>
  fleet.map((choice) => {
    const pose = glides.get(choice.id)?.at(nowNs);
    return pose ? { ...choice, latitude: pose.latitude, longitude: pose.longitude, heading: pose.heading } : choice;
  });
