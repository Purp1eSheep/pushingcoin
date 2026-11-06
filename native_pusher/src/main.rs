mod audio;
mod camera;
mod geometry;
mod particles;
mod physics;
mod renderer;

use std::sync::Arc;
use std::time::Instant;
use glam::Vec3;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use audio::SoundEngine;
use camera::{CameraController, CameraPreset};
use particles::ParticleSystem;
use physics::{CoinType, PhysicsWorld};
use renderer::VulkanRenderer;

struct App {
    window: Option<Arc<Window>>,
    renderer: Option<VulkanRenderer>,
    physics: PhysicsWorld,
    camera: CameraController,
    particles: ParticleSystem,
    sound: SoundEngine,

    spout_x: f32,
    auto_drop: bool,
    auto_drop_interval_ms: u64,
    last_auto_drop_time: Instant,
    last_frame_time: Instant,

    shake_timer: f32,
    shake_intensity: f32,

    last_title_update: Instant,
    frame_count: u32,
    fps: u32,
}

impl App {
    fn new() -> Self {
        // Read CLI options for side barrier default setting
        let args: Vec<String> = std::env::args().collect();
        let barriers_enabled = !args.iter().any(|a| {
            a == "--no-barriers"
                || a == "--open-sides"
                || a == "--barriers=false"
                || a == "--no-guards"
        });

        let mut physics = PhysicsWorld::new();
        physics.set_side_barriers(barriers_enabled);

        Self {
            window: None,
            renderer: None,
            physics,
            camera: CameraController::new(),
            particles: ParticleSystem::new(),
            sound: SoundEngine::new(),
            spout_x: 0.0,
            auto_drop: false,
            auto_drop_interval_ms: 750,
            last_auto_drop_time: Instant::now(),
            last_frame_time: Instant::now(),
            shake_timer: 0.0,
            shake_intensity: 0.0,
            last_title_update: Instant::now(),
            frame_count: 0,
            fps: 60,
        }
    }

    fn trigger_screen_shake(&mut self, intensity: f32, duration: f32) {
        self.shake_intensity = intensity;
        self.shake_timer = duration;
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let win_attr = Window::default_attributes()
            .with_title("🪙 頂級廉價多巴胺推幣機 (Rust + Vulkan 60 FPS)")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0));

        let window = Arc::new(
            event_loop
                .create_window(win_attr)
                .expect("Failed to create winit window"),
        );

        let renderer = pollster::block_on(VulkanRenderer::new(window.clone()));

        self.window = Some(window);
        self.renderer = Some(renderer);
        self.last_frame_time = Instant::now();
        self.last_title_update = Instant::now();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(new_size.width, new_size.height);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    self.camera.is_dragging = state == ElementState::Pressed;
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let pos = (position.x as f32, position.y as f32);
                if self.camera.is_dragging {
                    let dx = pos.0 - self.camera.last_mouse_pos.0;
                    let dy = pos.1 - self.camera.last_mouse_pos.1;
                    self.camera.on_mouse_drag(dx, dy);
                }
                self.camera.last_mouse_pos = pos;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll_y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => (p.y as f32) * 0.05,
                };
                self.camera.on_zoom(scroll_y);
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key_code),
                        state: ElementState::Pressed,
                        repeat,
                        ..
                    },
                ..
            } => match key_code {
                KeyCode::Space => {
                    if let Some(_) = self.physics.drop_coin(self.spout_x, None) {
                        self.sound.play_coin_drop();
                        self.trigger_screen_shake(0.03, 0.08);
                    }
                }
                KeyCode::KeyT => {
                    // Drop a rare Diamond coin!
                    if let Some(_) = self.physics.drop_coin(self.spout_x, Some(CoinType::Diamond)) {
                        self.sound.play_gem_score();
                        self.trigger_screen_shake(0.04, 0.10);
                    }
                }
                KeyCode::KeyA => {
                    if !repeat {
                        self.auto_drop = !self.auto_drop;
                        if self.auto_drop {
                            let interval = if self.physics.is_fever {
                                (self.auto_drop_interval_ms / 2).max(180)
                            } else {
                                self.auto_drop_interval_ms
                            };
                            println!("🎰 自動投幣：【開啟】（間隔：{} ms / 每秒 {:.1} 枚，按 [ / ] 鍵可調速）", interval, 1000.0 / interval as f32);
                        } else {
                            println!("⏹️ 自動投幣：【關閉】");
                        }
                    }
                }
                KeyCode::BracketLeft => {
                    if !repeat {
                        self.auto_drop_interval_ms = (self.auto_drop_interval_ms + 100).min(3000);
                        println!(
                            "⏱️ [設定] 自動投幣放慢：{} ms（約每秒 {:.1} 枚）",
                            self.auto_drop_interval_ms,
                            1000.0 / self.auto_drop_interval_ms as f32
                        );
                    }
                }
                KeyCode::BracketRight => {
                    if !repeat {
                        self.auto_drop_interval_ms = self.auto_drop_interval_ms.saturating_sub(100).max(200);
                        println!(
                            "⏱️ [設定] 自動投幣加快：{} ms（約每秒 {:.1} 枚）",
                            self.auto_drop_interval_ms,
                            1000.0 / self.auto_drop_interval_ms as f32
                        );
                    }
                }
                KeyCode::KeyS => {
                    if !repeat {
                        self.physics.trigger_shake();
                        self.sound.play_tilt();
                        self.trigger_screen_shake(0.22, 0.55);
                    }
                }
                KeyCode::KeyR => {
                    if !repeat {
                        self.physics.refill();
                        self.sound.play_fever();
                    }
                }
                KeyCode::KeyM => {
                    if !repeat {
                        self.sound.muted = !self.sound.muted;
                    }
                }
                KeyCode::KeyB | KeyCode::KeyW => {
                    if !repeat {
                        let state = self.physics.toggle_side_barriers();
                        self.sound.play_tilt();
                        if state {
                            println!("🛡️ [設定] 兩側隔板：【已升起開啟】（防護模式：阻擋硬幣側向掉落）");
                        } else {
                            println!("⚠️ [設定] 兩側隔板：【已降下收納】（真實街機模式：硬幣會掉入兩側落幣水溝）");
                        }
                    }
                }
                // Camera Presets & Interactive Adjustments
                KeyCode::Digit1 => {
                    self.camera.set_preset(CameraPreset::Classic);
                    println!("📷 鏡頭切換：【1. 經典街機視角】");
                }
                KeyCode::Digit2 => {
                    self.camera.set_preset(CameraPreset::TopDown);
                    println!("📷 鏡頭切換：【2. 俯瞰推幣視角】");
                }
                KeyCode::Digit3 => {
                    self.camera.set_preset(CameraPreset::CloseUp);
                    println!("📷 鏡頭切換：【3. 懸崖特寫視角】");
                }
                KeyCode::Digit4 => {
                    self.camera.set_preset(CameraPreset::Showcase);
                    println!("📷 鏡頭切換：【4. 全景外殼展示視角】");
                }
                KeyCode::KeyC => {
                    if !repeat {
                        let p = self.camera.cycle_preset();
                        println!("📷 鏡頭循環切換：【{}】", p.name());
                    }
                }
                KeyCode::ArrowUp => {
                    self.camera.adjust_pitch(3.0);
                }
                KeyCode::ArrowDown => {
                    self.camera.adjust_pitch(-3.0);
                }
                KeyCode::ArrowLeft => {
                    self.camera.adjust_yaw(-4.0);
                }
                KeyCode::ArrowRight => {
                    self.camera.adjust_yaw(4.0);
                }
                KeyCode::PageUp | KeyCode::Equal => {
                    self.camera.on_zoom(1.0);
                }
                KeyCode::PageDown | KeyCode::Minus => {
                    self.camera.on_zoom(-1.0);
                }
                KeyCode::Home => {
                    self.camera.set_preset(CameraPreset::Classic);
                    println!("📷 鏡頭已重設為經典街機視角");
                }
                KeyCode::Escape => {
                    event_loop.exit();
                }
                _ => {}
            },
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = now.duration_since(self.last_frame_time).as_secs_f32().min(0.05);
                self.last_frame_time = now;

                // Update camera smooth interpolation
                self.camera.update(dt);

                // Update moving spout along X
                self.spout_x = (self.physics.time * 1.5).sin() * 2.2;

                // Auto drop loop (Only when player explicitly turns on Auto-drop with 'A')
                let auto_drop_threshold = if self.physics.is_fever {
                    (self.auto_drop_interval_ms / 2).max(180)
                } else {
                    self.auto_drop_interval_ms
                };
                if self.auto_drop
                    && now.duration_since(self.last_auto_drop_time).as_millis() as u64 >= auto_drop_threshold
                {
                    if let Some(_) = self.physics.drop_coin(self.spout_x, None) {
                        self.sound.play_coin_drop();
                        self.trigger_screen_shake(0.02, 0.06);
                    } else {
                        self.auto_drop = false;
                        println!("⚠️ 硬幣已用盡，自動投幣已停止（按 R 鍵免費補充）");
                    }
                    self.last_auto_drop_time = now;
                }

                // Step Rapier3D physics
                let step_res = self.physics.step(dt);

                // Handle score events & celebrate
                if !step_res.scored_events.is_empty() {
                    for ev in &step_res.scored_events {
                        let count = match ev.coin_type {
                            CoinType::Diamond => 30,
                            CoinType::Ruby => 25,
                            _ => 15,
                        };
                        self.particles.burst(
                            Vec3::new(ev.position.x, -1.8, 3.8),
                            count,
                            self.physics.is_fever,
                            Some(ev.color),
                        );

                        match ev.coin_type {
                            CoinType::Diamond | CoinType::Ruby => {
                                self.sound.play_gem_score();
                            }
                            _ => {
                                self.sound.play_coin_score(self.physics.combo);
                            }
                        }
                    }

                    if self.physics.combo >= 10 {
                        self.trigger_screen_shake(0.12, 0.3);
                    }
                }

                if step_res.fever_started {
                    self.sound.play_fever();
                    self.trigger_screen_shake(0.25, 0.8);
                    self.particles.burst(Vec3::new(0.0, 1.2, 3.4), 100, true, None);
                }

                // Update particles
                self.particles.update(dt);

                // Update screen shake decay
                let mut shake_offset = Vec3::ZERO;
                if self.shake_timer > 0.0 {
                    self.shake_timer -= dt;
                    let decay = (self.shake_timer / 0.5).clamp(0.0, 1.0);
                    let intensity = self.shake_intensity * decay;
                    shake_offset = Vec3::new(
                        (self.physics.time * 55.0).sin() * intensity,
                        (self.physics.time * 42.0).cos() * intensity * 0.7,
                        0.0,
                    );
                }

                // FPS & stats calculation
                self.frame_count += 1;
                if now.duration_since(self.last_title_update).as_secs_f32() >= 0.25 {
                    let elapsed = now.duration_since(self.last_title_update).as_secs_f32();
                    self.fps = (self.frame_count as f32 / elapsed).round() as u32;
                    self.frame_count = 0;
                    self.last_title_update = now;

                    if let Some(window) = &self.window {
                        let cam_str = format!(" | 📷 {}", self.camera.active_preset.name());
                        let title = if self.physics.is_fever {
                            format!(
                                "🚨🔥 [HYPER FEVER 狂熱模式 5X!] 剩餘: {:.1}s | FPS: {} | 💰 餘額: {} | 🏆 得分: {} | 🪙 幣山: {}{} | 🔥 {}x COMBO! 🚨",
                                self.physics.fever_time_remaining,
                                self.fps,
                                self.physics.balance,
                                self.physics.score,
                                self.physics.coins.len(),
                                cam_str,
                                self.physics.combo,
                            )
                        } else {
                            // Fever Progress Bar
                            let bars = (self.physics.fever_meter / 10.0).round() as usize;
                            let bar_str = "█".repeat(bars) + &"░".repeat(10 - bars);
                            let combo_str = if self.physics.combo > 1 {
                                format!(" | 🔥 {}x COMBO!", self.physics.combo)
                            } else {
                                String::new()
                            };
                            let auto_str = if self.auto_drop {
                                let cur_interval = if self.physics.is_fever {
                                    (self.auto_drop_interval_ms / 2).max(180)
                                } else {
                                    self.auto_drop_interval_ms
                                };
                                format!(" | 🎰 自動連投: ON ({}ms)", cur_interval)
                            } else {
                                String::new()
                            };
                            let barrier_str = if self.physics.side_barriers_enabled {
                                " | 🛡️ 隔板: ON"
                            } else {
                                " | ⚠️ 隔板: OFF(開放側溝)"
                            };
                            format!(
                                "🪙 Vulkan 推幣機 | FPS: {} | 💰 餘額: {} | 🏆 得分: {}{}{}{}{} | ⚡ FEVER [{}] {:.0}%{}{}",
                                self.fps,
                                self.physics.balance,
                                self.physics.score,
                                barrier_str,
                                " [B切換]",
                                cam_str,
                                " [1-4/C/拖曳]",
                                bar_str,
                                self.physics.fever_meter,
                                combo_str,
                                auto_str
                            )
                        };
                        window.set_title(&title);
                    }
                }

                // Render Vulkan frame
                if let Some(renderer) = &mut self.renderer {
                    let transforms = self.physics.get_coin_transforms();
                    let pusher_mat = self.physics.get_pusher_transform();
                    let particle_instances = self.particles.get_instances();
                    let cam_eye = self.camera.get_eye();
                    let cam_target = self.camera.get_target();
                    let cam_fov = self.camera.get_fov();

                    match renderer.render(
                        &transforms,
                        pusher_mat,
                        self.spout_x,
                        self.physics.side_barriers_enabled,
                        &particle_instances,
                        self.physics.time,
                        self.physics.is_fever,
                        self.physics.fever_meter,
                        self.physics.combo,
                        shake_offset,
                        cam_eye,
                        cam_target,
                        cam_fov,
                    ) {
                        Ok(_) => {}
                        Err(renderer::RenderError::Lost) => {
                            let (w, h) = renderer.size;
                            renderer.resize(w, h);
                        }
                        Err(renderer::RenderError::OutOfMemory) => {
                            event_loop.exit();
                        }
                        Err(e) => {
                            log::warn!("Render error: {:?}", e);
                        }
                    }
                }

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn main() {
    env_logger::init();

    println!("============================================================");
    println!("🪙 頂級廉價多巴胺推幣機 (High-Dopamine Rust + Vulkan Arcade)");
    println!("============================================================");
    println!("🎮 豐富操作說明：");
    println!("  [空白鍵 Space] - 投擲硬幣 (隨機金幣/銀幣/紅寶石/鑽石)");
    println!("  [T 鍵]         - 💎 投擲稀有鑽石幣 (Diamond Coin)");
    println!("  [A 鍵]         - ⚡ 切換自動節奏投幣 (預設 750ms/枚，舒緩適中)");
    println!("  [[ / ] 鍵]     - ⏱️ 減速 / 加速自動投幣間隔 (+/- 100ms 微調)");
    println!("  [B / W 鍵]     - 🛡️ 開關兩側隔板 (切換 防掉隔板 / 真實街機落幣側溝)");
    println!("  [S 鍵]         - 💥 震撼搖晃機台 (Tilt Shake & 物理震波)");
    println!("  [R 鍵]         - 🎁 免費補充 50 枚硬幣 (Refill)");
    println!("  [M 鍵]         - 🔇 靜音 / 開啟音效 (Mute Toggle)");
    println!("------------------------------------------------------------");
    println!("📷 鏡頭操作視角 (Camera Controls)：");
    println!("  [1 / 2 / 3 / 4] - 📐 快速切換 4 大鏡頭預設視角：");
    println!("                     1: 🎮 經典街機 (Classic Arcade) - 全局推台與掉落視角");
    println!("                     2: 🦅 俯瞰推幣 (Top-Down Deck)  - 專注雙層滑台推動節奏");
    println!("                     3: 🔍 懸崖特寫 (Cliff Edge)     - 邊緣幣山即將崩塌特寫");
    println!("                     4: 👑 全景外殼 (Cabinet Showcase)- 炫彩霓虹跑馬燈機台全景");
    println!("  [C 鍵]         - 🔄 依序循環切換預設視角");
    println!("  [滑鼠左鍵拖曳] - 🔄 自由圍繞旋轉視角 (Orbit Controls)");
    println!("  [滾輪 / PgUp/Dn / +/-] - 🔍 縮放鏡頭前後距離 (Zoom In / Out)");
    println!("  [方向鍵 ↑↓←→]  - 📐 微調鏡頭俯仰與偏航角度 (Pitch / Yaw)");
    println!("  [Home 鍵]      - 🏠 一鍵重設回預設經典街機鏡頭視角");
    println!("  [ESC 鍵]       - 🚪 退出遊戲");
    println!("============================================================");
    println!("⚙️ 啟動設定參數 (CLI Options)：");
    println!("  --no-barriers / --open-sides  以開放式側溝啟動 (預設關閉兩側隔板)");
    println!("============================================================");
    println!("🔥 FEVER 狂熱機制：");
    println!("  每掉落一枚硬幣蓄力 +4.5%，蓄滿 100% 觸發 10 秒 HYPER FEVER！");
    println!("  狂熱期間：5倍得分加成！免費投幣不扣餘額！彩虹霓虹閃爍！");
    println!("============================================================");

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new();
    let _ = event_loop.run_app(&mut app);
}
