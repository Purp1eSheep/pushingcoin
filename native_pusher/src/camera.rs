use glam::Vec3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CameraPreset {
    Classic = 1,  // 1: 經典街機視角 (Classic Arcade)
    TopDown = 2,  // 2: 俯瞰推幣視角 (Top-down Deck Focus)
    CloseUp = 3,  // 3: 邊緣特寫視角 (Cliff Edge Tension)
    Showcase = 4, // 4: 全景外殼視角 (Full Cabinet Showcase)
}

impl CameraPreset {
    pub fn name(&self) -> &'static str {
        match self {
            CameraPreset::Classic => "🎮 經典街機",
            CameraPreset::TopDown => "🦅 俯瞰推幣",
            CameraPreset::CloseUp => "🔍 懸崖特寫",
            CameraPreset::Showcase => "👑 全景外殼",
        }
    }

    pub fn params(&self) -> (Vec3, f32, f32, f32, f32) {
        // (target, distance, pitch_deg, yaw_deg, fov_deg)
        match self {
            CameraPreset::Classic => (
                Vec3::new(0.0, 0.68, 1.65),
                8.0,
                34.0,
                0.0,
                45.0,
            ),
            CameraPreset::TopDown => (
                Vec3::new(0.0, 0.45, 1.50),
                7.1,
                56.0,
                0.0,
                43.0,
            ),
            CameraPreset::CloseUp => (
                Vec3::new(0.0, 0.20, 3.80),
                4.2,
                24.0,
                0.0,
                46.0,
            ),
            CameraPreset::Showcase => (
                Vec3::new(0.0, 1.15, 1.65),
                11.0,
                30.0,
                0.0,
                48.0,
            ),
        }
    }
}

pub struct CameraController {
    // Current animated values
    pub target: Vec3,
    pub distance: f32,
    pub pitch: f32, // in radians
    pub yaw: f32,   // in radians
    pub fov: f32,   // in radians

    // Desired target values (for smooth exponential lerping)
    pub dest_target: Vec3,
    pub dest_distance: f32,
    pub dest_pitch: f32,
    pub dest_yaw: f32,
    pub dest_fov: f32,

    pub active_preset: CameraPreset,

    // Mouse interaction
    pub is_dragging: bool,
    pub last_mouse_pos: (f32, f32),
}

impl CameraController {
    pub fn new() -> Self {
        let (target, distance, pitch_deg, yaw_deg, fov_deg) = CameraPreset::Classic.params();
        let pitch = pitch_deg.to_radians();
        let yaw = yaw_deg.to_radians();
        let fov = fov_deg.to_radians();

        Self {
            target,
            distance,
            pitch,
            yaw,
            fov,
            dest_target: target,
            dest_distance: distance,
            dest_pitch: pitch,
            dest_yaw: yaw,
            dest_fov: fov,
            active_preset: CameraPreset::Classic,
            is_dragging: false,
            last_mouse_pos: (0.0, 0.0),
        }
    }

    pub fn set_preset(&mut self, preset: CameraPreset) {
        self.active_preset = preset;
        let (target, distance, pitch_deg, yaw_deg, fov_deg) = preset.params();
        self.dest_target = target;
        self.dest_distance = distance;
        self.dest_pitch = pitch_deg.to_radians();
        self.dest_yaw = yaw_deg.to_radians();
        self.dest_fov = fov_deg.to_radians();
    }

    pub fn cycle_preset(&mut self) -> CameraPreset {
        let next = match self.active_preset {
            CameraPreset::Classic => CameraPreset::TopDown,
            CameraPreset::TopDown => CameraPreset::CloseUp,
            CameraPreset::CloseUp => CameraPreset::Showcase,
            CameraPreset::Showcase => CameraPreset::Classic,
        };
        self.set_preset(next);
        next
    }

    pub fn on_mouse_drag(&mut self, dx: f32, dy: f32) {
        let sensitivity = 0.005;
        self.dest_yaw -= dx * sensitivity;
        self.dest_pitch += dy * sensitivity;

        // Clamp pitch to avoid gimbal flip: between 10 deg and 85 deg
        let min_pitch = 10.0_f32.to_radians();
        let max_pitch = 85.0_f32.to_radians();
        self.dest_pitch = self.dest_pitch.clamp(min_pitch, max_pitch);

        // Clamp yaw to reasonable arcade viewing bounds: -75 to +75 deg
        let max_yaw = 75.0_f32.to_radians();
        self.dest_yaw = self.dest_yaw.clamp(-max_yaw, max_yaw);
    }

    pub fn on_zoom(&mut self, delta_y: f32) {
        self.dest_distance -= delta_y * 0.6;
        self.dest_distance = self.dest_distance.clamp(3.0, 18.0);
    }

    pub fn adjust_pitch(&mut self, delta_deg: f32) {
        self.dest_pitch += delta_deg.to_radians();
        let min_pitch = 10.0_f32.to_radians();
        let max_pitch = 85.0_f32.to_radians();
        self.dest_pitch = self.dest_pitch.clamp(min_pitch, max_pitch);
    }

    pub fn adjust_yaw(&mut self, delta_deg: f32) {
        self.dest_yaw += delta_deg.to_radians();
        let max_yaw = 75.0_f32.to_radians();
        self.dest_yaw = self.dest_yaw.clamp(-max_yaw, max_yaw);
    }

    pub fn update(&mut self, dt: f32) {
        // Smooth exponential interpolation (lerp) towards destination
        let blend = (1.0 - (-12.0 * dt).exp()).clamp(0.0, 1.0);
        self.target = self.target.lerp(self.dest_target, blend);
        self.distance += (self.dest_distance - self.distance) * blend;
        self.pitch += (self.dest_pitch - self.pitch) * blend;
        self.yaw += (self.dest_yaw - self.yaw) * blend;
        self.fov += (self.dest_fov - self.fov) * blend;
    }

    pub fn get_eye(&self) -> Vec3 {
        let x = self.target.x + self.distance * self.pitch.cos() * self.yaw.sin();
        let y = self.target.y + self.distance * self.pitch.sin();
        let z = self.target.z + self.distance * self.pitch.cos() * self.yaw.cos();
        Vec3::new(x, y, z)
    }

    pub fn get_target(&self) -> Vec3 {
        self.target
    }

    pub fn get_fov(&self) -> f32 {
        self.fov
    }
}
