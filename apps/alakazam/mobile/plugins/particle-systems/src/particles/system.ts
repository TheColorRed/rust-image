import type { ImageLike } from '@abra/core/image';
import type { SwarmLike } from '@abra/core/particles';
import type { BlendLike, SpritesLike } from '@abra/core/sprites';
import { release } from '@alakazam/sdk';
import type { MadeSystem } from './definition';

/**
 * A particle system with what does not change as it goes through its life already worked out, which `particleSprites` draws at a moment of that life.
 * The library does the work: where every particle is born and what it picks at random are decided once, here, by the library's `ParticleSystem`.
 */
export type PreparedParticles = {
  swarm: SwarmLike;
  /** The pictures the groups of the swarm are drawn as, by group number. A group without one is a soft dot. */
  textures: (ImageLike | undefined)[];
  blend: BlendLike;
  /** How many seconds the life lasts when the system makes particles over time, or 0 when it does not. */
  duration: number;
  dispose: () => void;
};

/**
 * Works out what does not change as the system goes through its life, once for a photo. The result is kept for as long as the system is the same, and
 * `particleSprites` then draws it at any progress without going back to the library for each particle. `photo` is the picture the particles are for: its
 * size, and its pixels for a place that is its bright parts. The caller lets the result go with `dispose`, and the system it was made from with its own.
 */
export const prepareParticles = (made: MadeSystem, photo: ImageLike): PreparedParticles => {
  const swarm = made.system.swarm(photo);
  const textures = made.system.textures().map((texture: ImageLike | undefined | null) => texture ?? undefined);
  return {
    swarm,
    textures,
    blend: made.blend,
    duration: made.duration,
    dispose: () => release(swarm, ...textures.filter((texture: ImageLike | undefined): texture is ImageLike => texture !== undefined)),
  };
};

/**
 * The particles of a prepared system `progress` (0 to 1) of the way through its life, as sprites for the library to draw: a `Sprites` effect, which
 * is an effect like any other, so it can be applied to a picture or set as a live effect on a layer, which the engine draws on its own thread each time
 * it composes. The library works out every particle in this one call. The caller lets the effect go with `release`.
 */
export const particleSprites = (prepared: PreparedParticles, progress: number): SpritesLike =>
  // A system that makes particles over time is drawn at a time in seconds, and `progress` is how far through its duration that is.
  prepared.swarm.sprites(prepared.duration > 0 ? progress * prepared.duration : progress, prepared.blend, prepared.textures);
