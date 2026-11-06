import type { CoinData } from './types';

/**
 * Generate authentic pre-populated coin scatter on both tiers:
 * - Upper deck (Pusher / Shelf): ~48 coins
 * - Lower deck (Main table): ~52 coins
 * Total ~100 coins forming a dramatic, teetering arcade pile!
 */
export function generatePrepopulatedCoins(): CoinData[] {
  const coins: CoinData[] = [];

  // 1. Upper Pusher / Shelf Deck Coins (48 coins)
  const upperCount = 48;
  for (let i = 0; i < upperCount; i++) {
    const layer = Math.floor(i / 16);
    const col = (i % 6) - 2.5;
    const row = Math.floor((i % 16) / 6);

    const x = col * 0.78 + (Math.random() - 0.5) * 0.15;
    const z = -0.85 + row * 0.42 + (Math.random() - 0.5) * 0.12;
    const y = 0.72 + layer * 0.085;
    const isSpecial = i === 12 || i === 34;

    coins.push({
      id: `pre-upper-${i}`,
      position: [x, y, z],
      rotation: [
        (Math.random() - 0.5) * 0.06,
        Math.random() * Math.PI,
        (Math.random() - 0.5) * 0.06,
      ],
      color: isSpecial ? '#ef4444' : i % 2 === 0 ? '#f59e0b' : '#fbbf24',
      isSpecial,
    });
  }

  // 2. Lower Deck Coins (84 coins spanning the extended runway from Z=0.9 to 5.6)
  const lowerCount = 84;
  for (let i = 0; i < lowerCount; i++) {
    const layer = Math.floor(i / 42);
    const indexInLayer = i % 42;
    const col = (indexInLayer % 6) - 2.5;
    const row = Math.floor(indexInLayer / 6);

    const x = col * 0.85 + (Math.random() - 0.5) * 0.18;
    const z = 0.95 + row * 0.68 + (Math.random() - 0.5) * 0.16;
    const y = 0.06 + layer * 0.085;
    const isSpecial = i === 7 || i === 25 || i === 42 || i === 68;

    coins.push({
      id: `pre-lower-${i}`,
      position: [x, y, z],
      rotation: [
        (Math.random() - 0.5) * 0.06,
        Math.random() * Math.PI,
        (Math.random() - 0.5) * 0.06,
      ],
      color: isSpecial ? '#06b6d4' : i % 3 === 0 ? '#f59e0b' : '#fbbf24',
      isSpecial,
    });
  }

  return coins;
}
