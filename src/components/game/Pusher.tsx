import { useRef } from 'react';
import type { FC } from 'react';
import { useFrame } from '@react-three/fiber';
import { RigidBody, CuboidCollider } from '@react-three/rapier';
import type { RapierRigidBody } from '@react-three/rapier';

export interface PusherProps {
  speed?: number;
  amplitude?: number;
  baseZ?: number;
  posY?: number;
}

/**
 * Pusher Component
 *
 * Designed with zero gaps throughout its entire stroke:
 * - The pusher block extends 3.4 units in depth.
 * - When at maximum forward stroke (Z = +0.05), its rear is still at Z = -2.0,
 *   which is 0.9 units behind the fixed scraper wall (Z = -1.1).
 * - There is NEVER an open trench or gap behind the pusher.
 * - Enabled CCD (Continuous Collision Detection) to prevent tunneling when coins are rapidly spammed.
 */
export const Pusher: FC<PusherProps> = ({
  speed = 1.2,
  amplitude = 0.75,
  baseZ = -0.7,
  posY = 0.33,
}) => {
  const pusherBodyRef = useRef<RapierRigidBody>(null);

  // Smoothly move the pusher back and forth along Z
  useFrame((state) => {
    if (!pusherBodyRef.current) return;
    const t = state.clock.getElapsedTime();
    const currentZ = baseZ + Math.sin(t * speed) * amplitude;
    pusherBodyRef.current.setNextKinematicTranslation({
      x: 0,
      y: posY,
      z: currentZ,
    });
  });

  return (
    <group>
      {/* ============================================================
          1. STATIONARY SCRAPER WALL (推板刮幣後壁)
          Positioned at Z = -1.1.
          Sits with 0.04 clearance directly above the sliding pusher plate.
          When the pusher retracts, coins on top hit this wall and are pushed
          forward off the top shelf when the pusher extends forward.
         ============================================================ */}
      <RigidBody
        type="fixed"
        colliders={false}
        position={[0, 1.35, -1.1]}
        friction={0.2}
        restitution={0.1}
      >
        <CuboidCollider args={[2.75, 0.65, 0.08]} friction={0.2} restitution={0.1} />
        {/* Scraper main barrier mesh */}
        <mesh castShadow receiveShadow>
          <boxGeometry args={[5.5, 1.3, 0.16]} />
          <meshStandardMaterial
            color="#0f172a"
            metalness={0.8}
            roughness={0.2}
          />
        </mesh>
        {/* Glowing neon accent trim on scraper blade */}
        <mesh position={[0, 0.63, 0.09]}>
          <boxGeometry args={[5.4, 0.06, 0.04]} />
          <meshStandardMaterial
            color="#ec4899"
            emissive="#ec4899"
            emissiveIntensity={2.5}
            toneMapped={false}
          />
        </mesh>
      </RigidBody>

      {/* ============================================================
          2. REAR MACHINERY HOUSING / ROOF (後方遮罩車頂 - 杜絕掉落硬幣進入後方機件)
          Covers the entire region behind Z = -1.1.
         ============================================================ */}
      <RigidBody
        type="fixed"
        colliders={false}
        position={[0, 2.05, -2.45]}
        friction={0.3}
        restitution={0.1}
      >
        <CuboidCollider args={[2.75, 0.06, 1.35]} friction={0.3} restitution={0.1} />
        <mesh receiveShadow castShadow>
          <boxGeometry args={[5.5, 0.12, 2.7]} />
          <meshStandardMaterial
            color="#1e293b"
            metalness={0.85}
            roughness={0.25}
          />
        </mesh>
        {/* Accent lines on roof plate */}
        <mesh position={[0, 0.07, 0]}>
          <boxGeometry args={[5.3, 0.02, 2.5]} />
          <meshStandardMaterial
            color="#0f172a"
            metalness={0.9}
            roughness={0.1}
          />
        </mesh>
      </RigidBody>

      {/* ============================================================
          3. UPPER SIDE GUARDS (推板兩側高導流板)
          Prevents coins on upper tier from spilling over the sides before the lower deck.
         ============================================================ */}
      <RigidBody
        type="fixed"
        colliders={false}
        position={[-2.82, 1.15, -1.3]}
        friction={0.2}
        restitution={0.1}
      >
        <CuboidCollider args={[0.08, 0.8, 2.0]} />
        <mesh castShadow receiveShadow>
          <boxGeometry args={[0.16, 1.6, 4.0]} />
          <meshStandardMaterial
            color="#1e293b"
            metalness={0.85}
            roughness={0.2}
          />
        </mesh>
      </RigidBody>

      <RigidBody
        type="fixed"
        colliders={false}
        position={[2.82, 1.15, -1.3]}
        friction={0.2}
        restitution={0.1}
      >
        <CuboidCollider args={[0.08, 0.8, 2.0]} />
        <mesh castShadow receiveShadow>
          <boxGeometry args={[0.16, 1.6, 4.0]} />
          <meshStandardMaterial
            color="#1e293b"
            metalness={0.85}
            roughness={0.2}
          />
        </mesh>
      </RigidBody>

      {/* ============================================================
          4. KINEMATIC MOVING PUSHER (無縫運動學推板)
          - RigidBody type="kinematicPosition" with ccd={true}
          - Extended depth (args=[2.68, 0.32, 1.7]) centered at Z = -0.6
          - Front face is at +1.10 relative to currentZ
          - Rear face is at -2.30 relative to currentZ
          - Even when max forward (currentZ = +0.05), rear face is at -2.25,
            well behind the scraper wall at -1.1. NO GAP EVER EXISTS!
         ============================================================ */}
      <RigidBody
        ref={pusherBodyRef}
        type="kinematicPosition"
        colliders={false}
        position={[0, posY, baseZ]}
        friction={0.45}
        restitution={0.0}
        ccd={true}
      >
        {/* Solid kinematic pusher collider with seamless extended rear */}
        <CuboidCollider
          position={[0, 0, -0.6]}
          args={[2.68, 0.32, 1.7]}
          friction={0.45}
          restitution={0.0}
        />

        {/* Pusher Body Meshes */}
        <group position={[0, 0, -0.6]}>
          {/* Main Heavy Steel Block */}
          <mesh castShadow receiveShadow>
            <boxGeometry args={[5.36, 0.64, 3.4]} />
            <meshStandardMaterial
              color="#334155"
              metalness={0.85}
              roughness={0.2}
            />
          </mesh>

          {/* Stainless Mirror Top Slider Plate */}
          <mesh position={[0, 0.325, 0]} rotation={[-Math.PI / 2, 0, 0]}>
            <planeGeometry args={[5.32, 3.36]} />
            <meshStandardMaterial
              color="#e2e8f0"
              metalness={0.96}
              roughness={0.08}
              envMapIntensity={1.8}
            />
          </mesh>

          {/* Polished Chrome Front Pushing Blade */}
          <mesh position={[0, 0, 1.705]} castShadow receiveShadow>
            <boxGeometry args={[5.36, 0.62, 0.05]} />
            <meshStandardMaterial
              color="#f8fafc"
              metalness={0.98}
              roughness={0.08}
            />
          </mesh>

          {/* Neon Edge Bar on Front Blade (Cyan Arcade Glow) */}
          <mesh position={[0, 0.30, 1.73]}>
            <boxGeometry args={[5.2, 0.05, 0.04]} />
            <meshStandardMaterial
              color="#06b6d4"
              emissive="#06b6d4"
              emissiveIntensity={2.8}
              toneMapped={false}
            />
          </mesh>

          {/* Bottom Front Skid Strip */}
          <mesh position={[0, -0.30, 1.73]}>
            <boxGeometry args={[5.2, 0.04, 0.04]} />
            <meshStandardMaterial
              color="#f59e0b"
              emissive="#f59e0b"
              emissiveIntensity={1.6}
              toneMapped={false}
            />
          </mesh>
        </group>
      </RigidBody>
    </group>
  );
};
