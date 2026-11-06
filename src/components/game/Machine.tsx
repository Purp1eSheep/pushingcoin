import { useCallback, useRef } from 'react';
import type { FC } from 'react';
import { RigidBody, CuboidCollider } from '@react-three/rapier';
import type { IntersectionEnterPayload } from '@react-three/rapier';

export interface MachineProps {
  onCoinDrop?: (coinId: string, edge: 'front' | 'side') => void;
  onScore?: (points: number, coinId: string) => void;
  enableSideWalls?: boolean;
}

export const Machine: FC<MachineProps> = ({
  onCoinDrop,
  onScore,
  enableSideWalls = true,
}) => {
  // Track handled coins to prevent double-firing in rapid succession
  const processedCoins = useRef<Set<string>>(new Set());

  const handleSensorIntersection = useCallback(
    (payload: IntersectionEnterPayload) => {
      const otherObject = payload.other.rigidBodyObject;
      const otherBody = payload.other.rigidBody;

      // Extract coin ID from userData
      const userData = (otherObject?.userData ?? otherBody?.userData) as
        | { id?: string; type?: string }
        | undefined;

      if (!userData || userData.type !== 'coin' || !userData.id) {
        return;
      }

      const coinId = userData.id;
      if (processedCoins.current.has(coinId)) {
        return;
      }
      processedCoins.current.add(coinId);

      // Determine whether it slipped off front (player win) or side (house edge)
      const translation = otherBody?.translation();
      let edge: 'front' | 'side' = 'front';

      if (translation) {
        if (Math.abs(translation.x) > 3.0 && translation.z < 3.4) {
          edge = 'side';
        } else {
          edge = 'front';
        }
      }

      // Calculate score points (front drop gives 100 points, side drop gives 20 points)
      const points = edge === 'front' ? 100 : 20;

      onCoinDrop?.(coinId, edge);
      onScore?.(points, coinId);
    },
    [onCoinDrop, onScore]
  );

  return (
    <group>
      {/* ============================================================
          1. LOWER DECK (下層底板)
          Fixed RigidBody with friction 0.4 and restitution 0.1.
          Crucial rule: NO side walls and NO front wall. Coins can freely
          spill off Left (-X), Right (+X), and Front (+Z)!
          Dimensions: Width 6.4 (X: -3.2 ~ +3.2), Depth 4.8 (Z: -1.2 ~ +3.6)
          Top surface sits cleanly at Y = 0.0.
         ============================================================ */}
      <RigidBody
        type="fixed"
        colliders={false}
        position={[0, -0.2, 2.4]}
        friction={0.4}
        restitution={0.0}
      >
        <CuboidCollider args={[3.2, 0.2, 3.6]} friction={0.4} restitution={0.0} />

        {/* Lower Deck Main Body */}
        <mesh receiveShadow castShadow>
          <boxGeometry args={[6.4, 0.4, 7.2]} />
          <meshStandardMaterial
            color="#1e293b"
            metalness={0.7}
            roughness={0.3}
          />
        </mesh>

        {/* Polished Mirror-Finish Deck Surface */}
        <mesh position={[0, 0.201, 0]} rotation={[-Math.PI / 2, 0, 0]} receiveShadow>
          <planeGeometry args={[6.36, 7.16]} />
          <meshStandardMaterial
            color="#cbd5e1"
            metalness={0.92}
            roughness={0.15}
            envMapIntensity={1.8}
          />
        </mesh>

        {/* Front Edge Chamfer Accent Strip */}
        <mesh position={[0, 0.18, 3.61]}>
          <boxGeometry args={[6.38, 0.04, 0.02]} />
          <meshStandardMaterial
            color="#f59e0b"
            emissive="#f59e0b"
            emissiveIntensity={1.2}
            toneMapped={false}
          />
        </mesh>

        {/* Left Edge Chamfer Accent Strip */}
        <mesh position={[-3.21, 0.18, 0]}>
          <boxGeometry args={[0.02, 0.04, 7.18]} />
          <meshStandardMaterial
            color="#f59e0b"
            emissive="#f59e0b"
            emissiveIntensity={1.0}
            toneMapped={false}
          />
        </mesh>

        {/* Right Edge Chamfer Accent Strip */}
        <mesh position={[3.21, 0.18, 0]}>
          <boxGeometry args={[0.02, 0.04, 7.18]} />
          <meshStandardMaterial
            color="#f59e0b"
            emissive="#f59e0b"
            emissiveIntensity={1.0}
            toneMapped={false}
          />
        </mesh>
      </RigidBody>

      {/* ============================================================
          1.5. SIDE PANELS & AIR WALLS (側板與防脫軌空氣牆)
          - Left & Right Acrylic Side Panels: Prevent coins from tumbling off the sides into void
          - Front Retention Air Wall: Prevents coins from popping forward toward player/camera
          - Ceiling Air Wall: Prevents coins from launching into outer space
          - Rear Machinery Air Wall: Prevents coins from glitching into back mechanisms
         ============================================================ */}

      {/* Left & Right Acrylic Side Panels (Can be toggled via enableSideWalls setting) */}
      {enableSideWalls && (
        <>
          {/* Left Acrylic Side Panel (左側透明壓克力側板 + 物理剛體) */}
          <RigidBody
            type="fixed"
            colliders={false}
            position={[-3.24, 1.2, 2.4]}
            friction={0.2}
            restitution={0.1}
          >
            <CuboidCollider args={[0.06, 1.2, 3.6]} friction={0.2} restitution={0.1} />
            {/* Semi-transparent Cyan Tinted Acrylic Glass */}
            <mesh castShadow receiveShadow>
              <boxGeometry args={[0.06, 2.4, 7.2]} />
              <meshStandardMaterial
                color="#38bdf8"
                transparent
                opacity={0.32}
                roughness={0.1}
                metalness={0.3}
              />
            </mesh>
            {/* Top Glowing Cyan Trim Rail */}
            <mesh position={[0, 1.21, 0]}>
              <boxGeometry args={[0.08, 0.04, 7.2]} />
              <meshStandardMaterial
                color="#38bdf8"
                emissive="#38bdf8"
                emissiveIntensity={2.5}
                toneMapped={false}
              />
            </mesh>
          </RigidBody>

          {/* Right Acrylic Side Panel (右側透明壓克力側板 + 物理剛體) */}
          <RigidBody
            type="fixed"
            colliders={false}
            position={[3.24, 1.2, 2.4]}
            friction={0.2}
            restitution={0.1}
          >
            <CuboidCollider args={[0.06, 1.2, 3.6]} friction={0.2} restitution={0.1} />
            {/* Semi-transparent Magenta Tinted Acrylic Glass */}
            <mesh castShadow receiveShadow>
              <boxGeometry args={[0.06, 2.4, 7.2]} />
              <meshStandardMaterial
                color="#ec4899"
                transparent
                opacity={0.32}
                roughness={0.1}
                metalness={0.3}
              />
            </mesh>
            {/* Top Glowing Magenta Trim Rail */}
            <mesh position={[0, 1.21, 0]}>
              <boxGeometry args={[0.08, 0.04, 7.2]} />
              <meshStandardMaterial
                color="#ec4899"
                emissive="#ec4899"
                emissiveIntensity={2.5}
                toneMapped={false}
              />
            </mesh>
          </RigidBody>
        </>
      )}

      {/* Front Retention Air Wall (前方防噴飛空氣牆 / 前擋玻璃)
          Leaves Y: [0.0, 0.6] open at the bottom so coins slide off the front edge into the prize bin,
          while blocking stacked coins from bursting out forward towards the camera! */}
      <RigidBody
        type="fixed"
        colliders={false}
        position={[0, 2.2, 6.05]}
        friction={0.1}
        restitution={0.2}
      >
        <CuboidCollider args={[3.3, 1.5, 0.05]} />
        {/* Subtle clear glass pane with faint arcade reflection */}
        <mesh>
          <boxGeometry args={[6.5, 3.0, 0.02]} />
          <meshStandardMaterial
            color="#e0f2fe"
            transparent
            opacity={0.14}
            roughness={0.05}
            metalness={0.5}
          />
        </mesh>
        {/* Top arcade glass frame border */}
        <mesh position={[0, 1.51, 0]}>
          <boxGeometry args={[6.5, 0.05, 0.06]} />
          <meshStandardMaterial
            color="#64748b"
            metalness={0.9}
            roughness={0.2}
          />
        </mesh>
      </RigidBody>

      {/* Ceiling Air Wall (頂部防爆衝空氣牆 - 防止強烈碰撞時硬幣衝出宇宙) */}
      <RigidBody
        type="fixed"
        colliders={false}
        position={[0, 4.6, 1.5]}
      >
        <CuboidCollider args={[3.8, 0.08, 5.5]} />
      </RigidBody>

      {/* Rear Machinery Air Wall (後方機械防夾防卡空氣牆 - 封死推板後方滑台頂部縫隙) */}
      <RigidBody
        type="fixed"
        colliders={false}
        position={[0, 2.8, -2.6]}
      >
        <CuboidCollider args={[3.0, 1.2, 0.1]} />
      </RigidBody>


      {/* ============================================================
          2. BACK WALL (後方擋板)
          Fixed RigidBody behind the machine to prevent coins from falling backward.
          Positioned at Z = -3.8, spanning Y: [0, 5.2].
         ============================================================ */}
      <RigidBody
        type="fixed"
        colliders={false}
        position={[0, 2.6, -3.8]}
        friction={0.3}
        restitution={0.1}
      >
        <CuboidCollider args={[3.8, 2.6, 0.2]} friction={0.3} restitution={0.1} />

        {/* Back Wall Mesh with Retro Arcade Mirror Theme */}
        <mesh receiveShadow castShadow>
          <boxGeometry args={[7.6, 5.2, 0.4]} />
          <meshStandardMaterial
            color="#090d16"
            metalness={0.8}
            roughness={0.3}
          />
        </mesh>

        {/* Reflective Inner Mirror Surface */}
        <mesh position={[0, 0, 0.21]}>
          <planeGeometry args={[7.2, 4.8]} />
          <meshStandardMaterial
            color="#1e1b4b"
            metalness={0.95}
            roughness={0.1}
            envMapIntensity={2.0}
          />
        </mesh>

        {/* Retro Grid Lines on Back Mirror */}
        {[-1.5, -0.5, 0.5, 1.5].map((yOffset, idx) => (
          <mesh key={`grid-h-${idx}`} position={[0, yOffset, 0.22]}>
            <planeGeometry args={[7.0, 0.03]} />
            <meshStandardMaterial
              color="#38bdf8"
              emissive="#38bdf8"
              emissiveIntensity={1.8}
              toneMapped={false}
            />
          </mesh>
        ))}
        {[-2.5, -1.25, 0, 1.25, 2.5].map((xOffset, idx) => (
          <mesh key={`grid-v-${idx}`} position={[xOffset, 0, 0.22]}>
            <planeGeometry args={[0.03, 4.6]} />
            <meshStandardMaterial
              color="#a855f7"
              emissive="#a855f7"
              emissiveIntensity={1.8}
              toneMapped={false}
            />
          </mesh>
        ))}
      </RigidBody>

      {/* ============================================================
          3. COLLECTION SENSORS (專屬出幣口獲獎感應區)
          Specifically positioned inside the front winning chute (Z: 5.6 ~ 7.2, Y: -1.8 ~ -0.6).
          This prevents coins inside the machine or on decks from accidentally vanishing!
         ============================================================ */}
      <RigidBody
        type="fixed"
        colliders={false}
        position={[0, -1.2, 6.4]}
      >
        <CuboidCollider
          args={[3.4, 0.6, 0.8]}
          sensor
          onIntersectionEnter={handleSensorIntersection}
        />
      </RigidBody>

      {/* Deep Out-of-Bounds Abyss Catchment (深淵防卡死清理感應區 - 僅處理跌落世界底部的硬幣) */}
      <RigidBody
        type="fixed"
        colliders={false}
        position={[0, -8.0, 0]}
      >
        <CuboidCollider
          args={[25, 1.0, 25]}
          sensor
          onIntersectionEnter={handleSensorIntersection}
        />
      </RigidBody>


      {/* ============================================================
          4. ARCADE CABINET CHASSIS & VINTAGE AESTHETICS (機台外殼與霓虹質感)
         ============================================================ */}
      <group>
        {/* Slanted Lower Base Cabinet */}
        <mesh position={[0, -1.5, 2.4]} receiveShadow>
          <boxGeometry args={[7.4, 2.2, 8.0]} />
          <meshStandardMaterial
            color="#0b0f19"
            metalness={0.6}
            roughness={0.4}
          />
        </mesh>

        {/* Front Coin Recovery Chute (Where winning coins drop) */}
        <group position={[0, -0.8, 6.2]}>
          <mesh rotation={[Math.PI / 4, 0, 0]} receiveShadow>
            <boxGeometry args={[5.2, 0.1, 1.4]} />
            <meshStandardMaterial
              color="#334155"
              metalness={0.9}
              roughness={0.2}
            />
          </mesh>
          {/* Winner Coin Bin Lip */}
          <mesh position={[0, -0.4, 0.8]}>
            <boxGeometry args={[5.4, 0.5, 0.2]} />
            <meshStandardMaterial
              color="#e2e8f0"
              metalness={0.95}
              roughness={0.1}
            />
          </mesh>
          {/* Glowing "WINNER" Accent on Tray */}
          <mesh position={[0, -0.3, 0.91]}>
            <boxGeometry args={[3.2, 0.1, 0.02]} />
            <meshStandardMaterial
              color="#10b981"
              emissive="#10b981"
              emissiveIntensity={2.5}
              toneMapped={false}
            />
          </mesh>
        </group>

        {/* Left Side Lost Coin Gutter */}
        <mesh position={[-3.65, -0.7, 2.4]} rotation={[0, 0, -Math.PI / 5]} receiveShadow>
          <boxGeometry args={[0.9, 0.08, 7.2]} />
          <meshStandardMaterial
            color="#1e293b"
            metalness={0.8}
            roughness={0.3}
          />
        </mesh>

        {/* Right Side Lost Coin Gutter */}
        <mesh position={[3.65, -0.7, 2.4]} rotation={[0, 0, Math.PI / 5]} receiveShadow>
          <boxGeometry args={[0.9, 0.08, 7.2]} />
          <meshStandardMaterial
            color="#1e293b"
            metalness={0.8}
            roughness={0.3}
          />
        </mesh>

        {/* Vintage Neon Columns (左側與右側復古霓虹燈柱) */}
        {/* Left Neon Pillar */}
        <group position={[-3.8, 2.2, 5.6]}>
          {/* Outer Protective Glass Column */}
          <mesh>
            <cylinderGeometry args={[0.16, 0.16, 4.4, 16]} />
            <meshStandardMaterial
              color="#06b6d4"
              metalness={0.1}
              roughness={0.1}
              transparent
              opacity={0.35}
            />
          </mesh>
          {/* Inner High-Glow Neon Core */}
          <mesh>
            <cylinderGeometry args={[0.08, 0.08, 4.3, 16]} />
            <meshStandardMaterial
              color="#00f0ff"
              emissive="#00f0ff"
              emissiveIntensity={3.5}
              toneMapped={false}
            />
          </mesh>
          {/* Chrome Top & Bottom Brackets */}
          <mesh position={[0, 2.22, 0]}>
            <cylinderGeometry args={[0.2, 0.2, 0.16, 16]} />
            <meshStandardMaterial color="#f8fafc" metalness={0.95} roughness={0.1} />
          </mesh>
          <mesh position={[0, -2.22, 0]}>
            <cylinderGeometry args={[0.2, 0.2, 0.16, 16]} />
            <meshStandardMaterial color="#f8fafc" metalness={0.95} roughness={0.1} />
          </mesh>
        </group>

        {/* Right Neon Pillar */}
        <group position={[3.8, 2.2, 5.6]}>
          {/* Outer Protective Glass Column */}
          <mesh>
            <cylinderGeometry args={[0.16, 0.16, 4.4, 16]} />
            <meshStandardMaterial
              color="#ec4899"
              metalness={0.1}
              roughness={0.1}
              transparent
              opacity={0.35}
            />
          </mesh>
          {/* Inner High-Glow Neon Core */}
          <mesh>
            <cylinderGeometry args={[0.08, 0.08, 4.3, 16]} />
            <meshStandardMaterial
              color="#ff007f"
              emissive="#ff007f"
              emissiveIntensity={3.5}
              toneMapped={false}
            />
          </mesh>
          {/* Chrome Top & Bottom Brackets */}
          <mesh position={[0, 2.22, 0]}>
            <cylinderGeometry args={[0.2, 0.2, 0.16, 16]} />
            <meshStandardMaterial color="#f8fafc" metalness={0.95} roughness={0.1} />
          </mesh>
          <mesh position={[0, -2.22, 0]}>
            <cylinderGeometry args={[0.2, 0.2, 0.16, 16]} />
            <meshStandardMaterial color="#f8fafc" metalness={0.95} roughness={0.1} />
          </mesh>
        </group>

        {/* Top Header Marquee Beam */}
        <group position={[0, 4.5, 3.2]}>
          <mesh castShadow>
            <boxGeometry args={[8.0, 0.5, 0.6]} />
            <meshStandardMaterial
              color="#0f172a"
              metalness={0.9}
              roughness={0.2}
            />
          </mesh>
          {/* Arcade Crown Neon Strip */}
          <mesh position={[0, 0, 0.31]}>
            <boxGeometry args={[7.4, 0.14, 0.02]} />
            <meshStandardMaterial
              color="#fbbf24"
              emissive="#fbbf24"
              emissiveIntensity={2.5}
              toneMapped={false}
            />
          </mesh>
        </group>

        {/* Machine Roof Glass Cover (Decorative / Upper structure) */}
        <mesh position={[0, 4.8, -0.2]} rotation={[Math.PI / 16, 0, 0]}>
          <boxGeometry args={[7.6, 0.08, 6.8]} />
          <meshStandardMaterial
            color="#020617"
            metalness={0.8}
            roughness={0.2}
          />
        </mesh>
      </group>
    </group>
  );
};
