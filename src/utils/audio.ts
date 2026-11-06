/**
 * Web Audio API Sound Synthesis System for Coin Pusher
 * 100% pure procedural Web Audio API synthesis - zero external audio files, zero latency, 100% offline.
 */

export class AudioManager {
  private static instance: AudioManager | null = null;
  private ctx: AudioContext | null = null;
  private masterGain: GainNode | null = null;
  private muted: boolean = false;
  private volume: number = 0.5;
  private isUnlocked: boolean = false;
  private noiseBuffer: AudioBuffer | null = null;

  public constructor() {
    this.initUserGestureUnlock();
  }

  /**
   * Singleton instance accessor
   */
  public static getInstance(): AudioManager {
    if (!AudioManager.instance) {
      AudioManager.instance = new AudioManager();
    }
    return AudioManager.instance;
  }

  /**
   * Ensure AudioContext is initialized and available
   */
  private ensureContext(): AudioContext | null {
    if (typeof window === 'undefined') {
      return null;
    }

    if (!this.ctx) {
      const AudioCtxClass =
        window.AudioContext ||
        (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;

      if (!AudioCtxClass) {
        return null;
      }

      this.ctx = new AudioCtxClass();

      // Master gain node
      this.masterGain = this.ctx.createGain();
      this.masterGain.gain.setValueAtTime(this.muted ? 0 : this.volume, this.ctx.currentTime);
      this.masterGain.connect(this.ctx.destination);
    }

    if (this.ctx.state === 'suspended') {
      this.ctx.resume().catch(() => {
        // Autoplay policy prevented immediate resume; will resume on user gesture
      });
    }

    return this.ctx;
  }

  /**
   * Listen to first user gesture (click, keydown, touch) to unlock Web Audio API
   */
  private initUserGestureUnlock(): void {
    if (typeof window === 'undefined') return;

    const unlock = (): void => {
      if (this.isUnlocked) return;
      const ctx = this.ensureContext();
      if (ctx) {
        ctx
          .resume()
          .then(() => {
            this.isUnlocked = true;
          })
          .catch(() => {});
      }
    };

    const events = ['click', 'keydown', 'touchstart', 'pointerdown'];
    for (const evt of events) {
      window.addEventListener(evt, unlock, { once: true, passive: true });
    }
  }

  /**
   * Cached noise buffer for metallic strike transients
   */
  private getNoiseBuffer(ctx: AudioContext): AudioBuffer {
    if (!this.noiseBuffer || this.noiseBuffer.sampleRate !== ctx.sampleRate) {
      const bufferSize = Math.floor(ctx.sampleRate * 0.02); // 20ms burst
      const buffer = ctx.createBuffer(1, bufferSize, ctx.sampleRate);
      const data = buffer.getChannelData(0);
      for (let i = 0; i < bufferSize; i++) {
        data[i] = Math.random() * 2 - 1;
      }
      this.noiseBuffer = buffer;
    }
    return this.noiseBuffer;
  }

  /**
   * Set global mute state
   */
  public setMuted(muted: boolean): void {
    this.muted = muted;
    if (this.ctx && this.masterGain) {
      const now = this.ctx.currentTime;
      this.masterGain.gain.cancelScheduledValues(now);
      this.masterGain.gain.setValueAtTime(this.masterGain.gain.value, now);
      this.masterGain.gain.linearRampToValueAtTime(this.muted ? 0 : this.volume, now + 0.05);
    }
  }

  /**
   * Toggle global mute state
   */
  public toggleMute(): boolean {
    this.setMuted(!this.muted);
    return this.muted;
  }

  /**
   * Check if sound is currently muted
   */
  public isMuted(): boolean {
    return this.muted;
  }

  /**
   * Set master volume (0.0 to 1.0)
   */
  public setMasterVolume(vol: number): void {
    this.volume = Math.max(0, Math.min(1, vol));
    if (this.ctx && this.masterGain && !this.muted) {
      const now = this.ctx.currentTime;
      this.masterGain.gain.cancelScheduledValues(now);
      this.masterGain.gain.setValueAtTime(this.masterGain.gain.value, now);
      this.masterGain.gain.linearRampToValueAtTime(this.volume, now + 0.05);
    }
  }

  /**
   * Get current master volume
   */
  public getMasterVolume(): number {
    return this.volume;
  }

  /**
   * Play coin dropping into chute/track (Crisp high-frequency pitch slide)
   */
  public playCoinDrop(): void {
    const ctx = this.ensureContext();
    if (!ctx || !this.masterGain || this.muted) return;

    const now = ctx.currentTime;

    // High-pass filter to keep sound bright and remove mud
    const filter = ctx.createBiquadFilter();
    filter.type = 'highpass';
    filter.frequency.setValueAtTime(1400, now);
    filter.connect(this.masterGain);

    // Primary metallic slide chirp (2800Hz -> 1600Hz)
    const osc = ctx.createOscillator();
    const oscGain = ctx.createGain();

    osc.type = 'sine';
    osc.frequency.setValueAtTime(2800, now);
    osc.frequency.exponentialRampToValueAtTime(1600, now + 0.09);

    oscGain.gain.setValueAtTime(0.001, now);
    oscGain.gain.linearRampToValueAtTime(0.35, now + 0.005);
    oscGain.gain.exponentialRampToValueAtTime(0.001, now + 0.1);

    osc.connect(oscGain);
    oscGain.connect(filter);

    // Secondary high-frequency ring (4800Hz harmonic ping)
    const ringOsc = ctx.createOscillator();
    const ringGain = ctx.createGain();

    ringOsc.type = 'sine';
    ringOsc.frequency.setValueAtTime(4800, now);
    ringOsc.frequency.exponentialRampToValueAtTime(3200, now + 0.06);

    ringGain.gain.setValueAtTime(0.2, now);
    ringGain.gain.exponentialRampToValueAtTime(0.001, now + 0.05);

    ringOsc.connect(ringGain);
    ringGain.connect(filter);

    osc.start(now);
    ringOsc.start(now);
    osc.stop(now + 0.11);
    ringOsc.stop(now + 0.07);
  }

  /**
   * Play coin clinking against another coin
   * Inharmonic metallic partials + randomized pitch offset to prevent ear fatigue
   */
  public playCoinClink(): void {
    const ctx = this.ensureContext();
    if (!ctx || !this.masterGain || this.muted) return;

    const now = ctx.currentTime;

    // Pitch randomization (+-12%) for organic clatter variation
    const pitchOffset = (Math.random() - 0.5) * 0.24;
    const baseFreq = 3800 * (1 + pitchOffset);

    // Highpass filter for crisp metal clatter
    const filter = ctx.createBiquadFilter();
    filter.type = 'bandpass';
    filter.frequency.setValueAtTime(baseFreq, now);
    filter.Q.setValueAtTime(4, now);
    filter.connect(this.masterGain);

    // Transient strike impact (filtered noise click)
    const noiseSource = ctx.createBufferSource();
    noiseSource.buffer = this.getNoiseBuffer(ctx);
    const noiseGain = ctx.createGain();
    noiseGain.gain.setValueAtTime(0.25, now);
    noiseGain.gain.exponentialRampToValueAtTime(0.001, now + 0.012);
    noiseSource.connect(noiseGain);
    noiseGain.connect(filter);

    // Harmonic 1: Fundamental ping
    const osc1 = ctx.createOscillator();
    const gain1 = ctx.createGain();
    osc1.type = 'sine';
    osc1.frequency.setValueAtTime(baseFreq, now);
    gain1.gain.setValueAtTime(0.3, now);
    gain1.gain.exponentialRampToValueAtTime(0.001, now + 0.08);
    osc1.connect(gain1);
    gain1.connect(this.masterGain);

    // Harmonic 2: Inharmonic metal overtone (~1.58x)
    const osc2 = ctx.createOscillator();
    const gain2 = ctx.createGain();
    osc2.type = 'sine';
    osc2.frequency.setValueAtTime(baseFreq * 1.58, now);
    gain2.gain.setValueAtTime(0.18, now);
    gain2.gain.exponentialRampToValueAtTime(0.001, now + 0.05);
    osc2.connect(gain2);
    gain2.connect(this.masterGain);

    // Harmonic 3: High shimmer overtone (~2.32x)
    const osc3 = ctx.createOscillator();
    const gain3 = ctx.createGain();
    osc3.type = 'triangle';
    osc3.frequency.setValueAtTime(baseFreq * 2.32, now);
    gain3.gain.setValueAtTime(0.08, now);
    gain3.gain.exponentialRampToValueAtTime(0.001, now + 0.035);
    osc3.connect(gain3);
    gain3.connect(this.masterGain);

    noiseSource.start(now);
    osc1.start(now);
    osc2.start(now);
    osc3.start(now);

    noiseSource.stop(now + 0.015);
    osc1.stop(now + 0.09);
    osc2.stop(now + 0.06);
    osc3.stop(now + 0.04);
  }

  /**
   * Play coin falling over the edge into score hopper
   * Crisp celebratory chime; pitch increases dynamically with combo count!
   */
  public playCoinFallScore(combo: number = 1): void {
    const ctx = this.ensureContext();
    if (!ctx || !this.masterGain || this.muted) return;

    const now = ctx.currentTime;

    // Pentatonic scale frequencies for rewarding, uplifting progression
    const PENTATONIC_SCALE = [
      1046.5, // C6
      1174.7, // D6
      1318.5, // E6
      1568.0, // G6
      1760.0, // A6
      2093.0, // C7
      2349.3, // D7
      2637.0, // E7
      3136.0, // G7
      3520.0, // A7
      4186.0, // C8
    ];

    const safeCombo = Math.max(1, Math.floor(combo));
    const noteIdx = Math.min(safeCombo - 1, PENTATONIC_SCALE.length - 1);
    const rootFreq = PENTATONIC_SCALE[noteIdx];

    // Primary bell tone
    const osc1 = ctx.createOscillator();
    const gain1 = ctx.createGain();
    osc1.type = 'sine';
    osc1.frequency.setValueAtTime(rootFreq, now);

    gain1.gain.setValueAtTime(0.001, now);
    gain1.gain.linearRampToValueAtTime(0.35, now + 0.004);
    gain1.gain.exponentialRampToValueAtTime(0.001, now + 0.28);

    osc1.connect(gain1);
    gain1.connect(this.masterGain);

    // Bright octave overtone
    const osc2 = ctx.createOscillator();
    const gain2 = ctx.createGain();
    osc2.type = 'sine';
    osc2.frequency.setValueAtTime(rootFreq * 2, now);

    gain2.gain.setValueAtTime(0.001, now);
    gain2.gain.linearRampToValueAtTime(0.18, now + 0.004);
    gain2.gain.exponentialRampToValueAtTime(0.001, now + 0.16);

    osc2.connect(gain2);
    gain2.connect(this.masterGain);

    osc1.start(now);
    osc2.start(now);
    osc1.stop(now + 0.3);
    osc2.stop(now + 0.18);

    // Extra sparkle arpeggio note for higher combos (>= 3)
    if (safeCombo >= 3) {
      const sparkleOsc = ctx.createOscillator();
      const sparkleGain = ctx.createGain();
      const sparkleTime = now + 0.05;

      sparkleOsc.type = 'triangle';
      sparkleOsc.frequency.setValueAtTime(rootFreq * 1.5, sparkleTime);

      sparkleGain.gain.setValueAtTime(0.001, sparkleTime);
      sparkleGain.gain.linearRampToValueAtTime(0.2, sparkleTime + 0.003);
      sparkleGain.gain.exponentialRampToValueAtTime(0.001, sparkleTime + 0.22);

      sparkleOsc.connect(sparkleGain);
      sparkleGain.connect(this.masterGain);

      sparkleOsc.start(sparkleTime);
      sparkleOsc.stop(sparkleTime + 0.25);
    }

    // Super chime for high fever tier (>= 6)
    if (safeCombo >= 6) {
      const feverOsc = ctx.createOscillator();
      const feverGain = ctx.createGain();
      const feverTime = now + 0.09;

      feverOsc.type = 'sine';
      feverOsc.frequency.setValueAtTime(rootFreq * 2.5, feverTime);

      feverGain.gain.setValueAtTime(0.001, feverTime);
      feverGain.gain.linearRampToValueAtTime(0.15, feverTime + 0.003);
      feverGain.gain.exponentialRampToValueAtTime(0.001, feverTime + 0.24);

      feverOsc.connect(feverGain);
      feverGain.connect(this.masterGain);

      feverOsc.start(feverTime);
      feverOsc.stop(feverTime + 0.26);
    }
  }

  /**
   * Play euphoric Jackpot / Fever cascading chord fanfare
   * Ascending celebratory arpeggio followed by sustained golden major chord
   */
  public playJackpotCascade(): void {
    const ctx = this.ensureContext();
    if (!ctx || !this.masterGain || this.muted) return;

    const now = ctx.currentTime;

    // Fast ascending sparkling arpeggio: C5 -> E5 -> G5 -> B5 -> C6 -> E6 -> G6 -> C7
    const arpeggioNotes = [
      523.25,  // C5
      659.25,  // E5
      783.99,  // G5
      987.77,  // B5
      1046.50, // C6
      1318.51, // E6
      1567.98, // G6
      2093.00, // C7
    ];

    arpeggioNotes.forEach((freq, idx) => {
      const noteTime = now + idx * 0.065;
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();

      osc.type = 'sine';
      osc.frequency.setValueAtTime(freq, noteTime);

      gain.gain.setValueAtTime(0.001, noteTime);
      gain.gain.linearRampToValueAtTime(0.28, noteTime + 0.004);
      gain.gain.exponentialRampToValueAtTime(0.001, noteTime + 0.3);

      osc.connect(gain);
      gain.connect(this.masterGain!);

      osc.start(noteTime);
      osc.stop(noteTime + 0.32);
    });

    // Sustained triumphant major chord fanfare at climax (t + 0.55s)
    const chordTime = now + 0.55;
    const chordNotes = [
      1046.50, // C6
      1318.51, // E6
      1567.98, // G6
      2093.00, // C7
    ];

    chordNotes.forEach((freq) => {
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();

      osc.type = 'triangle';
      osc.frequency.setValueAtTime(freq, chordTime);

      // Add gentle pitch vibrato for lush arcade organ shimmer
      const lfo = ctx.createOscillator();
      const lfoGain = ctx.createGain();
      lfo.frequency.setValueAtTime(6, chordTime); // 6Hz vibrato
      lfoGain.gain.setValueAtTime(freq * 0.006, chordTime);
      lfo.connect(osc.frequency);
      lfo.start(chordTime);
      lfo.stop(chordTime + 0.9);

      gain.gain.setValueAtTime(0.001, chordTime);
      gain.gain.linearRampToValueAtTime(0.2, chordTime + 0.03);
      gain.gain.setValueAtTime(0.2, chordTime + 0.35);
      gain.gain.exponentialRampToValueAtTime(0.001, chordTime + 0.9);

      osc.connect(gain);
      gain.connect(this.masterGain!);

      osc.start(chordTime);
      osc.stop(chordTime + 0.95);
    });
  }

  /**
   * Play arcade machine tilt / nudge warning buzzer
   * Dual detuned aggressive sawtooth wave through lowpass filter with 2 sharp buzzer pulses
   */
  public playTiltBuzzer(): void {
    const ctx = this.ensureContext();
    if (!ctx || !this.masterGain || this.muted) return;

    const now = ctx.currentTime;

    const filter = ctx.createBiquadFilter();
    filter.type = 'lowpass';
    filter.frequency.setValueAtTime(950, now);
    filter.Q.setValueAtTime(2.5, now);
    filter.connect(this.masterGain);

    // Dual detuned low sawtooth oscillators (110Hz and 117Hz)
    const osc1 = ctx.createOscillator();
    const osc2 = ctx.createOscillator();
    const gainNode = ctx.createGain();

    osc1.type = 'sawtooth';
    osc1.frequency.setValueAtTime(110, now);

    osc2.type = 'sawtooth';
    osc2.frequency.setValueAtTime(117, now);

    osc1.connect(gainNode);
    osc2.connect(gainNode);
    gainNode.connect(filter);

    // Pulse 1: 0.00s -> 0.12s
    gainNode.gain.setValueAtTime(0.001, now);
    gainNode.gain.linearRampToValueAtTime(0.38, now + 0.005);
    gainNode.gain.setValueAtTime(0.38, now + 0.10);
    gainNode.gain.exponentialRampToValueAtTime(0.001, now + 0.12);

    // Pulse 2: 0.16s -> 0.28s
    gainNode.gain.setValueAtTime(0.001, now + 0.15);
    gainNode.gain.linearRampToValueAtTime(0.38, now + 0.165);
    gainNode.gain.setValueAtTime(0.38, now + 0.26);
    gainNode.gain.exponentialRampToValueAtTime(0.001, now + 0.29);

    osc1.start(now);
    osc2.start(now);
    osc1.stop(now + 0.31);
    osc2.stop(now + 0.31);
  }
}

// Global Singleton Instance
export const audioManager: AudioManager = AudioManager.getInstance();

// Exported standalone functions as requested
export const playCoinDrop = (): void => audioManager.playCoinDrop();
export const playCoinClink = (): void => audioManager.playCoinClink();
export const playCoinFallScore = (combo: number = 1): void => audioManager.playCoinFallScore(combo);
export const playJackpotCascade = (): void => audioManager.playJackpotCascade();
export const playTiltBuzzer = (): void => audioManager.playTiltBuzzer();
export const toggleMute = (): boolean => audioManager.toggleMute();
export const setMuted = (muted: boolean): void => audioManager.setMuted(muted);
export const isMuted = (): boolean => audioManager.isMuted();
export const setMasterVolume = (vol: number): void => audioManager.setMasterVolume(vol);
export const getMasterVolume = (): number => audioManager.getMasterVolume();
