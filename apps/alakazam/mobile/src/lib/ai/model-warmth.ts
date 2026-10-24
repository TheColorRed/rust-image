import { AiModel } from '@abra/core';
import { loadAiModelAsync } from '@alakazam/mobile';
import { release } from '@/src/lib/native/native-handles';

const models: Record<string, ReturnType<typeof AiModel.load>> = {};
const loads = new Map<string, Promise<ReturnType<typeof AiModel.load>>>();
const users = new Map<string, Set<string>>();
const activeRuns = new Map<string, number>();
const pendingUnload = new Set<string>();
const releasedOwners = new Map<string, Set<string>>();
const releaseTimers = new Map<string, ReturnType<typeof setTimeout>>();
const registrations = new Map<string, number>();

export type AiModelLifetime = {
  /** Called when the model is registered. Return true to retain it until `onRelease`. */
  retain?: boolean;
  /** Called after each run settles. Return true to release this registration. */
  releaseAfterRun?: (result: { failed: boolean }) => boolean;
  /** Called when the owner explicitly releases the registration. */
  onRelease?: () => void;
};

/** Built-in policies can be composed or replaced by a plugin without extending a central union. */
export const aiModelLifetimes = {
  oneShot: {
    retain: false,
    releaseAfterRun: () => true,
  } satisfies AiModelLifetime,
  multiShot: {
    retain: true,
  } satisfies AiModelLifetime,
} as const;

export type AiModelRegistration = {
  owner: string;
  role: string;
  lifetime: AiModelLifetime;
};

/** Loads a model for a tool and keeps it ready until that tool releases it. */
export const registerAiModel = ({ owner, role, lifetime }: AiModelRegistration): Promise<void> => {
  const key = `${owner}\0${role}`;
  const generation = (registrations.get(key) ?? 0) + 1;
  registrations.set(key, generation);
  releasedOwners.get(role)?.delete(owner);
  return load(role).then(() => {
    if (registrations.get(key) !== generation) return;
    if (releasedOwners.get(role)?.has(owner)) return Promise.resolve();
    const owners = users.get(role) ?? new Set<string>();
    if (lifetime.retain) owners.add(owner);
    users.set(role, owners);
    if (lifetime.onRelease) {
      const hooks = lifetimeReleaseHooks.get(role) ?? new Map<string, () => void>();
      hooks.set(owner, lifetime.onRelease);
      lifetimeReleaseHooks.set(role, hooks);
    }
  });
};

/** Releases a tool's model and frees it once no other tool or AI run needs it. */
export const releaseAiModel = (owner: string, role?: string): void => {
  const roles = new Set<string>([
    ...(role ? [role] : []),
    ...[...users.entries()].filter(([, owners]) => owners.has(owner)).map(([loadedRole]) => loadedRole),
    ...[...registrations.keys()].filter(key => key.startsWith(`${owner}\0`)).map(key => key.slice(owner.length + 1)),
  ]);
  if (role) registrations.delete(`${owner}\0${role}`);
  else for (const key of registrations.keys()) if (key.startsWith(`${owner}\0`)) registrations.delete(key);
  for (const loadedRole of roles) {
    const owners = users.get(loadedRole);
    const released = releasedOwners.get(loadedRole) ?? new Set<string>();
    released.add(owner);
    releasedOwners.set(loadedRole, released);
    owners?.delete(owner);
    lifetimeReleaseHooks.get(loadedRole)?.get(owner)?.();
    lifetimeReleaseHooks.get(loadedRole)?.delete(owner);
    if (owners?.size) continue;
    users.delete(loadedRole);
    if ((activeRuns.get(loadedRole) ?? 0) > 0 || loads.has(loadedRole)) pendingUnload.add(loadedRole);
    else deferUnload(loadedRole);
  }
};

const lifetimeReleaseHooks = new Map<string, Map<string, () => void>>();

/** Gets a warmed model and keeps it alive until the AI run settles. */
export const runWithAiModel = async <T>(
  registration: AiModelRegistration,
  run: (model: ReturnType<typeof AiModel.load>) => Promise<T>,
): Promise<T> => {
  const { owner, role, lifetime } = registration;
  activeRuns.set(role, (activeRuns.get(role) ?? 0) + 1);
  let failed = false;
  try {
    const loaded = await load(role);
    if (lifetime.retain && !users.get(role)?.has(owner)) await registerAiModel(registration);
    return await run(loaded);
  } catch (error) {
    failed = true;
    throw error;
  } finally {
    if (lifetime.releaseAfterRun?.({ failed })) releaseAiModel(owner, role);
    const remaining = (activeRuns.get(role) ?? 1) - 1;
    if (remaining) activeRuns.set(role, remaining);
    else {
      activeRuns.delete(role);
      if (pendingUnload.has(role) && !users.has(role)) unload(role);
    }
  }
};

const unload = (role: string): void => {
  if ((activeRuns.get(role) ?? 0) > 0 || users.has(role)) {
    pendingUnload.add(role);
    return;
  }
  release(models[role]);
  delete models[role];
  pendingUnload.delete(role);
  lifetimeReleaseHooks.delete(role);
  releasedOwners.delete(role);
};

const deferUnload = (role: string): void => {
  if (releaseTimers.has(role)) return;
  // Drop a heavy native model after the current UI interaction, never in the toolbar event handler.
  releaseTimers.set(role, setTimeout(() => {
    releaseTimers.delete(role);
    unload(role);
  }, 0));
};

const load = (role: string): Promise<ReturnType<typeof AiModel.load>> => {
  if (models[role]) return Promise.resolve(models[role]);
  let pending = loads.get(role);
  if (!pending) {
    pending = loadAiModelAsync(role)
      .then(model => {
        models[role] = model;
        if (pendingUnload.has(role) && !users.has(role) && (activeRuns.get(role) ?? 0) === 0) unload(role);
        return model as ReturnType<typeof AiModel.load>;
      })
      .finally(() => loads.delete(role));
    loads.set(role, pending);
  }
  return pending;
};

