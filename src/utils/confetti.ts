import confetti from 'canvas-confetti';

/**
 * Confetti Visual Effects System for Coin Pusher
 * Powered by canvas-confetti for high dopamine arcade visual feedback.
 */

// Active fever animation controller
let activeFeverAnimationId: number | null = null;
let feverTimeoutId: ReturnType<typeof setTimeout> | null = null;

// Palette definitions
const GOLDEN_JACKPOT_COLORS = [
  '#FFD700', // Pure Gold
  '#FFDF00', // Golden Yellow
  '#FFA500', // Orange Gold
  '#F59E0B', // Amber
  '#FFEAA7', // Warm Cream Gold
  '#FFF275', // Shimmer Gold
  '#00F0FF', // Cyan Sparkle Accent
  '#FF007F', // Neon Magenta Accent
  '#FFFFFF', // Diamond White
];

const FEVER_DOPAMINE_COLORS = [
  '#FF1493', // Deep Pink
  '#00FFFF', // Electric Cyan
  '#FFD700', // Gold
  '#76FF03', // Neon Lime
  '#FF007F', // Hot Pink
  '#7928CA', // Electric Purple
  '#FFA500', // Vivid Orange
  '#FFFFFF', // Pure White
];

/**
 * Trigger massive dual-cannon Jackpot confetti
 * Blasts dense golden coins, stars, and celebratory streamers from both bottom corners
 */
export function triggerJackpotConfetti(): void {
  // Wave 1: Immediate powerful cannons from left & right corners
  confetti({
    particleCount: 90,
    angle: 60,
    spread: 65,
    origin: { x: 0.05, y: 0.85 },
    colors: GOLDEN_JACKPOT_COLORS,
    shapes: ['star', 'circle', 'square'],
    scalar: 1.25,
    startVelocity: 55,
    decay: 0.9,
    ticks: 280,
    gravity: 1.1,
    zIndex: 9999,
  });

  confetti({
    particleCount: 90,
    angle: 120,
    spread: 65,
    origin: { x: 0.95, y: 0.85 },
    colors: GOLDEN_JACKPOT_COLORS,
    shapes: ['star', 'circle', 'square'],
    scalar: 1.25,
    startVelocity: 55,
    decay: 0.9,
    ticks: 280,
    gravity: 1.1,
    zIndex: 9999,
  });

  // Wave 2: Staggered second burst at +160ms with wider spread
  setTimeout(() => {
    confetti({
      particleCount: 65,
      angle: 70,
      spread: 80,
      origin: { x: 0.1, y: 0.8 },
      colors: GOLDEN_JACKPOT_COLORS,
      shapes: ['star', 'circle'],
      scalar: 1.1,
      startVelocity: 60,
      decay: 0.91,
      ticks: 260,
      zIndex: 9999,
    });

    confetti({
      particleCount: 65,
      angle: 110,
      spread: 80,
      origin: { x: 0.9, y: 0.8 },
      colors: GOLDEN_JACKPOT_COLORS,
      shapes: ['star', 'circle'],
      scalar: 1.1,
      startVelocity: 60,
      decay: 0.91,
      ticks: 260,
      zIndex: 9999,
    });
  }, 160);

  // Wave 3: Grand finale center fireworks fountain at +320ms
  setTimeout(() => {
    confetti({
      particleCount: 110,
      angle: 90,
      spread: 120,
      origin: { x: 0.5, y: 0.6 },
      colors: GOLDEN_JACKPOT_COLORS,
      shapes: ['star', 'circle', 'square'],
      scalar: 1.3,
      startVelocity: 48,
      decay: 0.89,
      ticks: 300,
      gravity: 0.95,
      zIndex: 9999,
    });
  }, 320);
}

/**
 * Stop active fever confetti animation stream
 */
export function stopFeverConfetti(): void {
  if (activeFeverAnimationId !== null) {
    cancelAnimationFrame(activeFeverAnimationId);
    activeFeverAnimationId = null;
  }
  if (feverTimeoutId !== null) {
    clearTimeout(feverTimeoutId);
    feverTimeoutId = null;
  }
}

/**
 * Trigger continuous dopamine confetti micro-bursts during Fever mode
 * Emits floaty, sparkling neon particles continuously for the given duration
 *
 * @param durationMs Duration in milliseconds (default: 3000ms)
 */
export function triggerFeverConfetti(durationMs: number = 3000): void {
  // Cancel any existing running fever animation to avoid particle flood
  stopFeverConfetti();

  const animationEnd = Date.now() + durationMs;
  let lastSpawn = 0;
  const spawnInterval = 120; // Fire a burst every 120ms for smooth, continuous dopamine

  const frame = (time: number): void => {
    if (Date.now() >= animationEnd) {
      stopFeverConfetti();
      return;
    }

    if (time - lastSpawn >= spawnInterval) {
      lastSpawn = time;

      // Left angle pop
      confetti({
        particleCount: 5,
        angle: 60,
        spread: 45,
        origin: { x: 0.05, y: 0.75 },
        colors: FEVER_DOPAMINE_COLORS,
        shapes: ['circle', 'star'],
        scalar: 0.95,
        startVelocity: 38,
        decay: 0.93,
        ticks: 200,
        gravity: 0.8,
        zIndex: 9999,
      });

      // Right angle pop
      confetti({
        particleCount: 5,
        angle: 120,
        spread: 45,
        origin: { x: 0.95, y: 0.75 },
        colors: FEVER_DOPAMINE_COLORS,
        shapes: ['circle', 'star'],
        scalar: 0.95,
        startVelocity: 38,
        decay: 0.93,
        ticks: 200,
        gravity: 0.8,
        zIndex: 9999,
      });

      // Random drifting center sparkle
      confetti({
        particleCount: 4,
        angle: 90,
        spread: 80,
        origin: {
          x: 0.2 + Math.random() * 0.6,
          y: 0.3 + Math.random() * 0.3,
        },
        colors: FEVER_DOPAMINE_COLORS,
        shapes: ['circle', 'star'],
        scalar: 1.0,
        startVelocity: 25,
        decay: 0.94,
        ticks: 180,
        gravity: 0.7,
        zIndex: 9999,
      });
    }

    activeFeverAnimationId = requestAnimationFrame(frame);
  };

  activeFeverAnimationId = requestAnimationFrame(frame);

  feverTimeoutId = setTimeout(() => {
    stopFeverConfetti();
  }, durationMs + 100);
}

/**
 * Reset and clear any currently active canvas confetti
 */
export function resetConfetti(): void {
  stopFeverConfetti();
  confetti.reset();
}
