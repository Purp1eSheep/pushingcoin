import { memo } from 'react';
import type { FC } from 'react';
import { RigidBody, CylinderCollider } from '@react-three/rapier';

export interface CoinProps {
  id: string;
  position: [number, number, number];
  rotation?: [x: number, y: number, z: number];
  color?: string;
  radius?: number;
  height?: number;
}

export const COIN_RADIUS = 0.35;
export const COIN_HEIGHT = 0.08;

const DEFAULT_ROTATION: [x: number, y: number, z: number] = [0, 0, 0];

export const Coin: FC<CoinProps> = memo(({
  id,
  position,
  rotation = DEFAULT_ROTATION,
  color = '#f59e0b',
  radius = COIN_RADIUS,
  height = COIN_HEIGHT,
}) => {
  return (
    <RigidBody
      type="dynamic"
      colliders={false}
      position={position}
      rotation={rotation}
      mass={1}
      restitution={0.0}
      friction={0.4}
      linearDamping={0.65}
      angularDamping={0.6}
      ccd={true}
      canSleep={true}
      userData={{ id, type: 'coin' }}
    >
      <CylinderCollider
        args={[height / 2, radius]}
        mass={1}
        restitution={0.0}
        friction={0.4}
      />

      {/* Main Coin Cylinder */}
      <mesh castShadow receiveShadow>
        <cylinderGeometry args={[radius, radius, height, 32]} />
        <meshStandardMaterial
          color={color}
          metalness={0.85}
          roughness={0.2}
          envMapIntensity={1.2}
        />
      </mesh>

      {/* Coin Top Embossed Rim & Core */}
      <mesh position={[0, height / 2 + 0.001, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <ringGeometry args={[radius * 0.72, radius * 0.94, 32]} />
        <meshStandardMaterial
          color="#d97706"
          metalness={0.9}
          roughness={0.15}
        />
      </mesh>

      <mesh position={[0, height / 2 + 0.001, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <circleGeometry args={[radius * 0.45, 16]} />
        <meshStandardMaterial
          color="#fbbf24"
          metalness={0.92}
          roughness={0.12}
        />
      </mesh>

      {/* Coin Bottom Embossed Rim & Core */}
      <mesh position={[0, -height / 2 - 0.001, 0]} rotation={[Math.PI / 2, 0, 0]}>
        <ringGeometry args={[radius * 0.72, radius * 0.94, 32]} />
        <meshStandardMaterial
          color="#d97706"
          metalness={0.9}
          roughness={0.15}
        />
      </mesh>

      <mesh position={[0, -height / 2 - 0.001, 0]} rotation={[Math.PI / 2, 0, 0]}>
        <circleGeometry args={[radius * 0.45, 16]} />
        <meshStandardMaterial
          color="#fbbf24"
          metalness={0.92}
          roughness={0.12}
        />
      </mesh>
    </RigidBody>
  );
});

Coin.displayName = 'Coin';
