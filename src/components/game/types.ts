export interface CoinData {
  id: string;
  position: [number, number, number];
  rotation?: [number, number, number];
  color?: string;
  isSpecial?: boolean;
}

export interface CoinSpawnerHandle {
  dropCoin: (customProps?: Partial<CoinData>) => string;
  getSpoutX: () => number;
  resetCoins: () => void;
  getCoinCount: () => number;
  removeCoin?: (coinId: string) => void;
}
