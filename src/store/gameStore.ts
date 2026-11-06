/**
 * Game State & Economy Store for "Cheap Dopamine Generator" (Coin Pusher)
 *
 * Implements a lightweight, zero-dependency, high-performance reactive store
 * compatible with both React (via useSyncExternalStore) and React Three Fiber / Rapier
 * 3D physics loops (via getState / subscribe / direct actions).
 */

import { useSyncExternalStore, useRef, useCallback } from 'react';
import {
  audioManager,
  playCoinDrop,
  playCoinFallScore,
  playJackpotCascade,
  playTiltBuzzer,
} from '../utils/audio.ts';
import {
  triggerFeverConfetti,
  triggerJackpotConfetti,
  stopFeverConfetti,
  resetConfetti,
} from '../utils/confetti.ts';

// Game Economy & Timing Configuration
export const DEFAULT_COINS = 60;
export const REFILL_COINS_AMOUNT = 50;
export const FEVER_PROGRESS_PER_COIN = 5;
export const FEVER_MAX_PROGRESS = 100;
export const FEVER_DURATION_MS = 10000;
export const COMBO_TIMEOUT_MS = 1500;
export const SHAKE_COOLDOWN_SECONDS = 15;

/**
 * Core game state properties
 */
export interface GameState {
  /** Current player coin balance (default: 60) */
  coins: number;
  /** Total score / pushed-off coins count */
  score: number;
  /** Current active consecutive drop combo count (resets if > 1.5s gap) */
  combo: number;
  /** Highest combo recorded in this session */
  maxCombo: number;
  /** Fever gauge progress (0 to 100, +5 per scored coin, triggers FEVER at 100) */
  feverProgress: number;
  /** Whether Fever mode is currently active (doubles score and coin payout for 10s) */
  isFeverMode: boolean;
  /** Shake machine cooldown countdown in seconds (0 when ready, up to 15) */
  shakeCooldown: number;
  /** Whether the machine can currently be shaken */
  canShake: boolean;
  /** Whether game audio is muted */
  isMuted: boolean;
  /** Remaining seconds of Fever mode (0 when inactive, 10 to 0) */
  feverTimeRemaining: number;
}

/**
 * Game actions and mutators
 */
export interface GameActions {
  /**
   * Deducts 1 coin from player balance to drop into machine.
   * Returns false if balance <= 0 (disallowed), otherwise returns true.
   */
  dropCoin: () => boolean;

  /**
   * Called when one or more coins fall over the front edge.
   * Increases score, updates combo (resets if > 1.5s gap), increases feverProgress,
   * and awards dropped coins back 1:1 (or 2x during Fever mode) to player balance.
   *
   * @param count Number of coins dropped (default: 1)
   */
  scoreCoin: (count?: number) => void;

  /**
   * Triggers machine shake physics if cooldown is ready.
   * Returns true if shake was successfully triggered, false if on cooldown.
   */
  triggerShake: () => boolean;

  /**
   * Free coin refill (+50 coins) when player is bankrupt.
   *
   * @param amount Optional custom amount (defaults to 50)
   */
  refillCoins: (amount?: number) => void;

  /**
   * Toggles global audio mute state.
   */
  toggleMute: () => boolean;

  /**
   * Explicitly sets audio mute state.
   */
  setMuted: (muted: boolean) => void;

  /**
   * Resets entire game state and active timers back to default.
   */
  resetGame: () => void;
}

/**
 * Combined store type including state and actions
 */
export type GameStore = GameState & GameActions;

/**
 * API for direct access outside React (e.g. in 3D physics loops, frame hooks)
 */
export interface GameStoreAPI extends GameActions {
  getState: () => GameStore;
  setState: (
    partial: Partial<GameState> | ((prev: GameStore) => Partial<GameState>)
  ) => void;
  subscribe: (listener: (state: GameStore) => void) => () => void;
}

export type UseGameStore = {
  (): GameStore;
  <T>(selector: (state: GameStore) => T): T;
} & GameStoreAPI;

const INITIAL_STATE: GameState = {
  coins: DEFAULT_COINS,
  score: 0,
  combo: 0,
  maxCombo: 0,
  feverProgress: 0,
  isFeverMode: false,
  shakeCooldown: 0,
  canShake: true,
  isMuted: false,
  feverTimeRemaining: 0,
};

// Internal active timer references
let comboTimer: ReturnType<typeof setTimeout> | null = null;
let feverTimeout: ReturnType<typeof setTimeout> | null = null;
let feverInterval: ReturnType<typeof setInterval> | null = null;
let shakeInterval: ReturnType<typeof setInterval> | null = null;

function clearComboTimer(): void {
  if (comboTimer !== null) {
    clearTimeout(comboTimer);
    comboTimer = null;
  }
}

function clearFeverTimers(): void {
  if (feverTimeout !== null) {
    clearTimeout(feverTimeout);
    feverTimeout = null;
  }
  if (feverInterval !== null) {
    clearInterval(feverInterval);
    feverInterval = null;
  }
}

function clearShakeTimer(): void {
  if (shakeInterval !== null) {
    clearInterval(shakeInterval);
    shakeInterval = null;
  }
}

// Store listeners and state container
const listeners = new Set<(state: GameStore) => void>();

function getState(): GameStore {
  return currentState;
}

function setState(
  partial: Partial<GameState> | ((prev: GameStore) => Partial<GameState>)
): void {
  const nextPartial = typeof partial === 'function' ? partial(currentState) : partial;
  if (!nextPartial || Object.keys(nextPartial).length === 0) {
    return;
  }

  let hasChanged = false;
  for (const key of Object.keys(nextPartial) as (keyof GameState)[]) {
    if (!Object.is(currentState[key], nextPartial[key])) {
      hasChanged = true;
      break;
    }
  }

  if (!hasChanged) {
    return;
  }

  currentState = {
    ...currentState,
    ...nextPartial,
  };

  listeners.forEach((listener) => {
    try {
      listener(currentState);
    } catch (err) {
      console.error('[gameStore] Listener error:', err);
    }
  });
}

function subscribe(listener: (state: GameStore) => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function dropCoin(): boolean {
  const state = getState();
  if (state.coins <= 0) {
    return false;
  }

  setState((prev) => {
    if (prev.coins <= 0) return {};
    return { coins: prev.coins - 1 };
  });

  playCoinDrop();
  return true;
}

function startFeverMode(
  scoreGain: number,
  coinsGain: number,
  newCombo: number,
  newMaxCombo: number
): void {
  clearFeverTimers();

  setState({
    score: scoreGain,
    coins: coinsGain,
    combo: newCombo,
    maxCombo: newMaxCombo,
    feverProgress: FEVER_MAX_PROGRESS,
    isFeverMode: true,
    feverTimeRemaining: Math.floor(FEVER_DURATION_MS / 1000),
  });

  playJackpotCascade();

  if (typeof window !== 'undefined') {
    triggerJackpotConfetti();
    triggerFeverConfetti(FEVER_DURATION_MS);
  }

  feverInterval = setInterval(() => {
    const current = getState();
    if (!current.isFeverMode) {
      clearFeverTimers();
      return;
    }
    const nextTime = current.feverTimeRemaining - 1;
    if (nextTime <= 0) {
      endFeverMode();
    } else {
      setState({ feverTimeRemaining: nextTime });
    }
  }, 1000);

  feverTimeout = setTimeout(() => {
    endFeverMode();
  }, FEVER_DURATION_MS);
}

function endFeverMode(): void {
  clearFeverTimers();
  if (typeof window !== 'undefined') {
    stopFeverConfetti();
  }
  setState({
    isFeverMode: false,
    feverProgress: 0,
    feverTimeRemaining: 0,
  });
}

function scoreCoin(count: number = 1): void {
  const safeCount = typeof count === 'number' && count > 0 ? count : 1;
  const state = getState();

  const isFever = state.isFeverMode;
  const multiplier = isFever ? 2 : 1;
  const gainedPoints = safeCount * multiplier;
  const gainedCoins = safeCount * multiplier;

  const newScore = state.score + gainedPoints;
  const newCoins = state.coins + gainedCoins;

  // Combo handling: consecutive score within 1.5s adds to combo, timeout resets combo to 0
  clearComboTimer();
  const newCombo = state.combo + safeCount;
  const newMaxCombo = Math.max(state.maxCombo, newCombo);

  comboTimer = setTimeout(() => {
    setState({ combo: 0 });
    comboTimer = null;
  }, COMBO_TIMEOUT_MS);

  // Play audio tuned to dynamic combo scale
  playCoinFallScore(newCombo);

  if (isFever) {
    setState({
      score: newScore,
      coins: newCoins,
      combo: newCombo,
      maxCombo: newMaxCombo,
      feverProgress: FEVER_MAX_PROGRESS,
    });
  } else {
    const newProgress = Math.min(
      FEVER_MAX_PROGRESS,
      state.feverProgress + safeCount * FEVER_PROGRESS_PER_COIN
    );

    if (newProgress >= FEVER_MAX_PROGRESS) {
      startFeverMode(newScore, newCoins, newCombo, newMaxCombo);
    } else {
      setState({
        score: newScore,
        coins: newCoins,
        combo: newCombo,
        maxCombo: newMaxCombo,
        feverProgress: newProgress,
      });
    }
  }
}

function triggerShake(): boolean {
  const state = getState();
  if (!state.canShake || state.shakeCooldown > 0) {
    return false;
  }

  clearShakeTimer();
  playTiltBuzzer();

  setState({
    canShake: false,
    shakeCooldown: SHAKE_COOLDOWN_SECONDS,
  });

  shakeInterval = setInterval(() => {
    const current = getState();
    const nextCooldown = current.shakeCooldown - 1;
    if (nextCooldown <= 0) {
      clearShakeTimer();
      setState({
        shakeCooldown: 0,
        canShake: true,
      });
    } else {
      setState({
        shakeCooldown: nextCooldown,
      });
    }
  }, 1000);

  return true;
}

function refillCoins(amount: number = REFILL_COINS_AMOUNT): void {
  const safeAmount = typeof amount === 'number' && amount > 0 ? amount : REFILL_COINS_AMOUNT;
  setState((prev) => ({
    coins: prev.coins <= 0 ? safeAmount : prev.coins + safeAmount,
  }));
}

function toggleMute(): boolean {
  const nextMuted = !getState().isMuted;
  audioManager.setMuted(nextMuted);
  setState({ isMuted: nextMuted });
  return nextMuted;
}

function setMuted(muted: boolean): void {
  audioManager.setMuted(muted);
  setState({ isMuted: muted });
}

function resetGame(): void {
  clearComboTimer();
  clearFeverTimers();
  clearShakeTimer();

  if (typeof window !== 'undefined') {
    resetConfetti();
  }

  audioManager.setMuted(INITIAL_STATE.isMuted);

  setState({
    ...INITIAL_STATE,
  });
}

const actions: GameActions = {
  dropCoin,
  scoreCoin,
  triggerShake,
  refillCoins,
  toggleMute,
  setMuted,
  resetGame,
};

let currentState: GameStore = {
  ...INITIAL_STATE,
  ...actions,
};

function shallowEqual<T>(a: T, b: T): boolean {
  if (Object.is(a, b)) return true;
  if (
    typeof a !== 'object' ||
    a === null ||
    typeof b !== 'object' ||
    b === null
  ) {
    return false;
  }
  const keysA = Object.keys(a) as (keyof T)[];
  const keysB = Object.keys(b) as (keyof T)[];
  if (keysA.length !== keysB.length) return false;
  for (let i = 0; i < keysA.length; i++) {
    const key = keysA[i];
    if (
      !Object.prototype.hasOwnProperty.call(b, key) ||
      !Object.is(a[key], b[key])
    ) {
      return false;
    }
  }
  return true;
}

const useGameStoreHook = function <T = GameStore>(
  selector?: (state: GameStore) => T
): T {
  const lastStateRef = useRef<GameStore | null>(null);
  const lastSelectedRef = useRef<T | null>(null);

  const getSnapshot = useCallback(() => {
    const state = getState();
    if (!selector) {
      return state as unknown as T;
    }
    if (lastStateRef.current === state && lastSelectedRef.current !== null) {
      return lastSelectedRef.current;
    }
    const nextSelected = selector(state);
    if (
      lastSelectedRef.current !== null &&
      shallowEqual(lastSelectedRef.current, nextSelected)
    ) {
      return lastSelectedRef.current;
    }
    lastStateRef.current = state;
    lastSelectedRef.current = nextSelected;
    return nextSelected;
  }, [selector]);

  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
};

export const useGameStore = Object.assign(useGameStoreHook, {
  getState,
  setState,
  subscribe,
  dropCoin,
  scoreCoin,
  triggerShake,
  refillCoins,
  toggleMute,
  setMuted,
  resetGame,
}) as UseGameStore;

export const gameStore: GameStoreAPI = {
  getState,
  setState,
  subscribe,
  dropCoin,
  scoreCoin,
  triggerShake,
  refillCoins,
  toggleMute,
  setMuted,
  resetGame,
};

export {
  dropCoin,
  scoreCoin,
  triggerShake,
  refillCoins,
  toggleMute,
  setMuted,
  resetGame,
  getState,
  setState,
  subscribe,
};

export default useGameStore;
