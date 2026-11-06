import {
  useState,
  useRef,
  useCallback,
  useImperativeHandle,
  forwardRef,
  useEffect,
} from 'react';
import { useFrame } from '@react-three/fiber';
import type * as THREE from 'three';
import { Coin } from './Coin';
import type { CoinData, CoinSpawnerHandle } from './types';
import { generatePrepopulatedCoins } from './utils';

export interface CoinSpawnerProps {
  spoutSpeed?: number;
  spoutRange?: number;
  dropTrigger?: number;
  enableKeyboard?: boolean;
  onDrop?: (coinId: string) => void;
  onCoinCountChange?: (count: number) => void;
}

export const CoinSpawner = forwardRef<CoinSpawnerHandle, CoinSpawnerProps>(
  (
    {
      spoutSpeed = 1.4,
      spoutRange = 2.1,
      dropTrigger = 0,
      enableKeyboard = true,
      onDrop,
      onCoinCountChange,
    },
    ref
  ) => {
    // Initial pre-population
    const [coins, setCoins] = useState<CoinData[]>(() => generatePrepopulatedCoins());
    const spoutGroupRef = useRef<THREE.Group>(null);
    const currentSpoutX = useRef(0);
    const lastTriggerRef = useRef(dropTrigger);

    // Notify coin count changes
    useEffect(() => {
      onCoinCountChange?.(coins.length);
    }, [coins.length, onCoinCountChange]);

    // Animate spout indicator smoothly left and right
    useFrame((state) => {
      const t = state.clock.getElapsedTime();
      const x = Math.sin(t * spoutSpeed) * spoutRange;
      currentSpoutX.current = x;
      if (spoutGroupRef.current) {
        spoutGroupRef.current.position.x = x;
      }
    });

    const lastDropTimeRef = useRef(0);
    const dropCascadeIndexRef = useRef(0);

    // Drop coin implementation
    const dropCoin = useCallback(
      (customProps?: Partial<CoinData>): string => {
        const now = performance.now();
        if (now - lastDropTimeRef.current < 280) {
          dropCascadeIndexRef.current = (dropCascadeIndexRef.current + 1) % 8;
        } else {
          dropCascadeIndexRef.current = 0;
        }
        lastDropTimeRef.current = now;

        const newId = `coin-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
        // Stagger spawn height and slight lateral position so spammed coins never overlap on spawn!
        const dropX =
          currentSpoutX.current +
          ((dropCascadeIndexRef.current % 3) - 1) * 0.08 +
          (Math.random() - 0.5) * 0.06;
        const dropY = 3.3 + dropCascadeIndexRef.current * 0.18;
        const dropZ = -0.75 + (Math.random() - 0.5) * 0.1;

        const newCoin: CoinData = {
          id: newId,
          position: [dropX, dropY, dropZ],
          rotation: [
            (Math.random() - 0.5) * 0.1,
            Math.random() * Math.PI,
            (Math.random() - 0.5) * 0.1,
          ],
          color: Math.random() > 0.1 ? '#fbbf24' : '#f59e0b',
          ...customProps,
        };

        setCoins((prev) => [...prev, newCoin]);

        onDrop?.(newId);
        return newId;
      },
      [onDrop]
    );

    // Remove coin method (called when coin is scored / collected by sensor)
    const removeCoin = useCallback((coinId: string) => {
      setCoins((prev) => prev.filter((c) => c.id !== coinId));
    }, []);

    const resetCoins = useCallback(() => {
      setCoins(generatePrepopulatedCoins());
    }, []);

    // Expose imperative handle
    useImperativeHandle(
      ref,
      () => ({
        dropCoin,
        getSpoutX: () => currentSpoutX.current,
        resetCoins,
        getCoinCount: () => coins.length,
        removeCoin,
      }),
      [dropCoin, resetCoins, coins.length, removeCoin]
    );

    // Watch dropTrigger prop changes
    useEffect(() => {
      if (dropTrigger > lastTriggerRef.current) {
        dropCoin();
        lastTriggerRef.current = dropTrigger;
      }
    }, [dropTrigger, dropCoin]);

    // Keyboard listener for Spacebar drop
    useEffect(() => {
      if (!enableKeyboard) return;

      const handleKeyDown = (e: KeyboardEvent) => {
        if (e.code === 'Space' && !e.repeat) {
          e.preventDefault();
          dropCoin();
        }
      };

      window.addEventListener('keydown', handleKeyDown);
      return () => window.removeEventListener('keydown', handleKeyDown);
    }, [enableKeyboard, dropCoin]);

    return (
      <group>
        {/* ============================================================
            Top Spout Guide Rail
           ============================================================ */}
        <group position={[0, 3.8, -0.75]}>
          {/* Main Metal Horizontal Crossrail */}
          <mesh castShadow>
            <boxGeometry args={[5.2, 0.08, 0.08]} />
            <meshStandardMaterial color="#64748b" metalness={0.9} roughness={0.2} />
          </mesh>

          {/* Glowing Indicator Backing Strip */}
          <mesh position={[0, 0.06, 0]}>
            <boxGeometry args={[4.8, 0.02, 0.06]} />
            <meshStandardMaterial
              color="#38bdf8"
              emissive="#38bdf8"
              emissiveIntensity={2.0}
              toneMapped={false}
            />
          </mesh>
        </group>

        {/* ============================================================
            Moving Spout Dropper Indicator (投幣口指示器)
            Automatically oscillates along X in useFrame
           ============================================================ */}
        <group ref={spoutGroupRef} position={[0, 3.6, -0.75]}>
          {/* Chrome Slider Bracket */}
          <mesh castShadow>
            <boxGeometry args={[0.5, 0.16, 0.3]} />
            <meshStandardMaterial color="#f8fafc" metalness={0.95} roughness={0.1} />
          </mesh>

          {/* Coin Entry Funnel / Chute */}
          <mesh position={[0, -0.16, 0]} castShadow>
            <cylinderGeometry args={[0.18, 0.28, 0.24, 16]} />
            <meshStandardMaterial
              color="#fbbf24"
              metalness={0.9}
              roughness={0.15}
            />
          </mesh>

          {/* Neon Target Aim Arrow / Indicator Down */}
          <mesh position={[0, -0.42, 0.12]}>
            <coneGeometry args={[0.1, 0.2, 4]} />
            <meshStandardMaterial
              color="#ef4444"
              emissive="#ef4444"
              emissiveIntensity={3.5}
              toneMapped={false}
            />
          </mesh>

          {/* Glowing Pulse Light at Spout Mouth */}
          <pointLight
            position={[0, -0.25, 0]}
            color="#fbbf24"
            intensity={8}
            distance={2.5}
          />
        </group>

        {/* ============================================================
            Render All Active Dynamic Coins in Physical World
           ============================================================ */}
        {coins.map((coin) => (
          <Coin
            key={coin.id}
            id={coin.id}
            position={coin.position}
            rotation={coin.rotation}
            color={coin.color}
          />
        ))}
      </group>
    );
  }
);

CoinSpawner.displayName = 'CoinSpawner';
