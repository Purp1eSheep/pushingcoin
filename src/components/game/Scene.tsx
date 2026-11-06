import { useRef, useImperativeHandle, forwardRef, useCallback } from 'react';
import type { CSSProperties, ReactNode } from 'react';
import { Canvas } from '@react-three/fiber';
import { OrbitControls, PerspectiveCamera } from '@react-three/drei';
import { Physics } from '@react-three/rapier';
import { Machine } from './Machine';
import { Pusher } from './Pusher';
import { CoinSpawner } from './CoinSpawner';
import type { CoinSpawnerHandle } from './types';

export interface SceneProps {
  onScore?: (points: number, coinId: string) => void;
  onCoinDrop?: (coinId: string, edge: 'front' | 'side') => void;
  onCoinCountChange?: (count: number) => void;
  dropTrigger?: number;
  enableKeyboard?: boolean;
  enableOrbitControls?: boolean;
  pusherSpeed?: number;
  className?: string;
  style?: CSSProperties;
  children?: ReactNode;
}

export const Scene = forwardRef<CoinSpawnerHandle, SceneProps>(
  (
    {
      onScore,
      onCoinDrop,
      onCoinCountChange,
      dropTrigger = 0,
      enableKeyboard = true,
      enableOrbitControls = true,
      pusherSpeed = 1.2,
      className,
      style,
      children,
    },
    ref
  ) => {
    const spawnerRef = useRef<CoinSpawnerHandle>(null);

    // Forward imperative handle so external components can call dropCoin(), etc.
    useImperativeHandle(
      ref,
      () => ({
        dropCoin: (customProps) => spawnerRef.current?.dropCoin(customProps) ?? '',
        getSpoutX: () => spawnerRef.current?.getSpoutX() ?? 0,
        resetCoins: () => spawnerRef.current?.resetCoins(),
        getCoinCount: () => spawnerRef.current?.getCoinCount() ?? 0,
        removeCoin: (id) => spawnerRef.current?.removeCoin?.(id),
      }),
      []
    );

    // Handler when a coin falls through the Collection Sensor
    const handleCoinDrop = useCallback(
      (coinId: string, edge: 'front' | 'side') => {
        // Destroy coin from physics scene
        spawnerRef.current?.removeCoin?.(coinId);
        onCoinDrop?.(coinId, edge);
      },
      [onCoinDrop]
    );

    const handleScore = useCallback(
      (points: number, coinId: string) => {
        onScore?.(points, coinId);
      },
      [onScore]
    );

    return (
      <div
        className={className}
        style={{
          width: '100%',
          height: '100%',
          position: 'relative',
          overflow: 'hidden',
          backgroundColor: '#05070d',
          ...style,
        }}
      >
        <Canvas
          shadows
          gl={{
            antialias: true,
            alpha: false,
            powerPreference: 'high-performance',
          }}
        >
          {/* ============================================================
              1. CAMERA: Fixed arcade perspective view adjusted for longer runway
             ============================================================ */}
          <PerspectiveCamera
            makeDefault
            position={[0, 11.5, 12.8]}
            fov={44}
            near={0.1}
            far={100}
          />

          {/* ============================================================
              2. ORBIT CONTROLS
              Configured with comfortable angles for viewing the extended machine
             ============================================================ */}
          {enableOrbitControls && (
            <OrbitControls
              target={[0, 0.8, 2.2]}
              minDistance={7}
              maxDistance={22}
              minPolarAngle={Math.PI / 6}
              maxPolarAngle={Math.PI / 2.1}
              minAzimuthAngle={-Math.PI / 3}
              maxAzimuthAngle={Math.PI / 3}
              enablePan={false}
              enableDamping
              dampingFactor={0.05}
            />
          )}

          {/* ============================================================
              3. LIGHTING (聚光燈 spotlight、環境光 ambientLight、霓虹投光)
             ============================================================ */}
          <ambientLight intensity={0.75} color="#f8fafc" />

          {/* Main Key Spotlight casting crisp shadows over extended deck */}
          <spotLight
            position={[0, 15, 6]}
            angle={0.7}
            penumbra={0.75}
            intensity={120}
            color="#ffffff"
            castShadow
            shadow-mapSize-width={1024}
            shadow-mapSize-height={1024}
            shadow-camera-near={1}
            shadow-camera-far={30}
            shadow-bias={-0.0001}
          />

          {/* Warm Golden Overhead Fill on Drop Target & Lower Runway */}
          <pointLight
            position={[0, 7.5, 2.0]}
            color="#fbbf24"
            intensity={25}
            distance={14}
          />

          {/* Left Neon Cyan Side Accent */}
          <pointLight
            position={[-4.5, 4.0, 3.5]}
            color="#00f0ff"
            intensity={18}
            distance={12}
          />

          {/* Right Neon Magenta Side Accent */}
          <pointLight
            position={[4.5, 4.0, 3.5]}
            color="#ff007f"
            intensity={18}
            distance={12}
          />

          {/* Machine Underglow */}
          <pointLight
            position={[0, -1.0, 3.5]}
            color="#10b981"
            intensity={14}
            distance={8}
          />

          {/* ============================================================
              4. RAPIER PHYSICS WORLD (Fixed 1/60s timestep to guarantee stack stability)
             ============================================================ */}
          <Physics gravity={[0, -22, 0]} timeStep={1 / 60} interpolate={true}>
            {/* Main Machine Structure (Lower Deck, Back Wall, Sensor, Cabinet) */}
            <Machine onCoinDrop={handleCoinDrop} onScore={handleScore} />

            {/* Moving Pusher & Upper Shelf */}
            <Pusher speed={pusherSpeed} />

            {/* Oscillating Spout & Dynamic Coins System */}
            <CoinSpawner
              ref={spawnerRef}
              dropTrigger={dropTrigger}
              enableKeyboard={enableKeyboard}
              onCoinCountChange={onCoinCountChange}
            />

            {children}
          </Physics>
        </Canvas>
      </div>
    );
  }
);

Scene.displayName = 'Scene';
