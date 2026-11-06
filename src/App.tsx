import { useState, useRef, useEffect, useCallback } from 'react';
import {
  Coins,
  Flame,
  RotateCcw,
  Volume2,
  VolumeX,
  Zap,
  Play,
  Square,
  HelpCircle,
  Trophy,
  Sparkles,
  Layers,
} from 'lucide-react';
import { Scene } from './components/game/Scene';
import type { CoinSpawnerHandle } from './components/game/types';
import { useGameStore } from './store/gameStore';

interface FloatingFeedback {
  id: string;
  text: string;
  color: string;
}

export default function App() {
  const sceneRef = useRef<CoinSpawnerHandle>(null);
  const [isScreenShaking, setIsScreenShaking] = useState(false);
  const [autoDrop, setAutoDrop] = useState(false);
  const [showHelp, setShowHelp] = useState(false);
  const [floatingTexts, setFloatingTexts] = useState<FloatingFeedback[]>([]);
  const [activeBoardCoinCount, setActiveBoardCoinCount] = useState(100);

  // Subscribe to reactive store
  const coins = useGameStore((s) => s.coins);
  const score = useGameStore((s) => s.score);
  const combo = useGameStore((s) => s.combo);
  const maxCombo = useGameStore((s) => s.maxCombo);
  const feverProgress = useGameStore((s) => s.feverProgress);
  const isFeverMode = useGameStore((s) => s.isFeverMode);
  const feverTimeRemaining = useGameStore((s) => s.feverTimeRemaining);
  const canShake = useGameStore((s) => s.canShake);
  const shakeCooldown = useGameStore((s) => s.shakeCooldown);
  const isMuted = useGameStore((s) => s.isMuted);

  const lastDropTimeRef = useRef(0);

  // Core drop handler with safety micro-throttle to ensure smooth physics delivery
  const handleDropCoin = useCallback(() => {
    const now = performance.now();
    if (now - lastDropTimeRef.current < 70) {
      return;
    }
    lastDropTimeRef.current = now;

    const success = useGameStore.getState().dropCoin();
    if (success) {
      sceneRef.current?.dropCoin();
    } else {
      setAutoDrop(false);
    }
  }, []);

  // Handle machine shake
  const handleTriggerShake = useCallback(() => {
    const shaken = useGameStore.getState().triggerShake();
    if (shaken) {
      setIsScreenShaking(true);
      setTimeout(() => setIsScreenShaking(false), 520);
    }
  }, []);

  // Keyboard shortcut listener: Space to drop, S to shake
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
        return;
      }

      if (e.code === 'Space') {
        e.preventDefault();
        handleDropCoin();
      } else if (e.code === 'KeyS' && !e.repeat) {
        e.preventDefault();
        handleTriggerShake();
      } else if (e.code === 'KeyA' && !e.repeat) {
        e.preventDefault();
        setAutoDrop((prev) => !prev);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleDropCoin, handleTriggerShake]);

  // Auto-drop loop
  useEffect(() => {
    if (!autoDrop) return;

    const interval = setInterval(() => {
      const currentCoins = useGameStore.getState().coins;
      if (currentCoins <= 0) {
        setAutoDrop(false);
      } else {
        handleDropCoin();
      }
    }, 280);

    return () => clearInterval(interval);
  }, [autoDrop, handleDropCoin]);


  // Handle scored coin event from physical sensor
  const handleCoinDrop = useCallback((_coinId: string, edge: 'front' | 'side') => {
    // "所有都歸玩家不要有擋板" - Every drop awards score & refills balance!
    useGameStore.getState().scoreCoin(1);

    // Floating text feedback for dopamine
    const textId = `${Date.now()}-${Math.random()}`;
    const isFever = useGameStore.getState().isFeverMode;
    const currentCombo = useGameStore.getState().combo;

    let text = isFever ? '+2 FEVER!' : '+1 COIN!';
    let color = isFever ? 'text-pink-400' : 'text-amber-300';

    if (currentCombo >= 5) {
      text = `🔥 x${currentCombo} MEGA!`;
      color = 'text-yellow-400';
    } else if (edge === 'side') {
      text = isFever ? '+2 SIDE CASCADE!' : '+1 SIDE WIN!';
      color = 'text-cyan-400';
    }

    setFloatingTexts((prev) => [...prev.slice(-6), { id: textId, text, color }]);

    setTimeout(() => {
      setFloatingTexts((prev) => prev.filter((item) => item.id !== textId));
    }, 950);
  }, []);

  const handleResetGame = () => {
    if (window.confirm('確定要重新開始遊戲並重置盤面嗎？')) {
      useGameStore.getState().resetGame();
      sceneRef.current?.resetCoins();
      setAutoDrop(false);
    }
  };

  return (
    <div
      className={`relative w-screen h-screen overflow-hidden select-none bg-slate-950 text-white ${
        isScreenShaking ? 'shake-active' : ''
      } ${isFeverMode ? 'rainbow-mode' : ''}`}
    >
      {/* ============================================================
          1. 3D RAPIER PHYSICS SCENE CANVAS
         ============================================================ */}
      <div className="absolute inset-0 z-0">
        <Scene
          ref={sceneRef}
          onCoinDrop={handleCoinDrop}
          enableKeyboard={false}
          onCoinCountChange={setActiveBoardCoinCount}
        />
      </div>

      {/* ============================================================
          2. FLOATING DOPAMINE SCORE POPUPS
         ============================================================ */}
      <div className="absolute inset-0 pointer-events-none z-20 flex items-center justify-center">
        <div className="flex flex-col items-center space-y-1">
          {floatingTexts.map((item) => (
            <div
              key={item.id}
              className={`float-text font-black text-2xl md:text-3xl tracking-wider drop-shadow-[0_2px_12px_rgba(0,0,0,0.9)] ${item.color}`}
            >
              {item.text}
            </div>
          ))}
        </div>
      </div>

      {/* ============================================================
          3. TOP ARCADE HUD HEADER
         ============================================================ */}
      <header className="absolute top-0 left-0 right-0 z-30 p-3 md:p-5 flex flex-wrap items-center justify-between pointer-events-none gap-2">
        {/* Left: Player Wallet & Current Coin Balance */}
        <div className="flex items-center space-x-3 pointer-events-auto">
          <div
            className={`flex items-center space-x-2.5 px-4 py-2 rounded-2xl border backdrop-blur-md transition-all ${
              coins > 0
                ? 'bg-amber-950/60 border-amber-500/50 glow-gold text-amber-200'
                : 'bg-red-950/80 border-red-500/80 text-red-200 animate-pulse'
            }`}
          >
            <Coins className={`w-7 h-7 ${coins > 0 ? 'text-amber-400' : 'text-red-400'}`} />
            <div>
              <div className="text-[10px] uppercase font-bold tracking-wider opacity-70">
                持有硬幣 (Balance)
              </div>
              <div className="text-2xl md:text-3xl font-black tabular-nums tracking-tight">
                {coins}
              </div>
            </div>
          </div>

          {/* Quick Refill Button when coins low */}
          {coins === 0 && (
            <button
              onClick={() => useGameStore.getState().refillCoins()}
              className="px-4 py-2 bg-gradient-to-r from-emerald-500 to-teal-400 text-slate-950 font-black rounded-xl shadow-lg shadow-emerald-500/40 hover:scale-105 active:scale-95 transition-transform flex items-center space-x-1.5 cursor-pointer animate-bounce"
            >
              <Sparkles className="w-5 h-5" />
              <span>免費補幣 +50</span>
            </button>
          )}
        </div>

        {/* Center: FEVER Mode Meter & Combo Banner */}
        <div className="flex flex-col items-center pointer-events-auto max-w-xs md:max-w-md w-full mx-auto order-3 md:order-2">
          {/* FEVER Progress Bar */}
          <div className="w-full bg-slate-900/80 border border-slate-700/60 rounded-full p-1 shadow-2xl backdrop-blur-md">
            <div className="relative h-4 rounded-full overflow-hidden bg-slate-950">
              <div
                className={`h-full transition-all duration-300 rounded-full ${
                  isFeverMode
                    ? 'bg-gradient-to-r from-pink-500 via-purple-500 to-cyan-400 glow-fever animate-pulse'
                    : 'bg-gradient-to-r from-amber-500 to-yellow-400'
                }`}
                style={{ width: `${isFeverMode ? 100 : feverProgress}%` }}
              />
              <div className="absolute inset-0 flex items-center justify-center text-[10px] font-black uppercase tracking-widest text-white drop-shadow">
                {isFeverMode
                  ? `💥 狂熱 FEVER 雙倍回饋中！(${feverTimeRemaining}s) 💥`
                  : `FEVER 能量槽: ${feverProgress}%`}
              </div>
            </div>
          </div>

          {/* Combo High-Energy Indicator */}
          {combo > 1 && (
            <div className="mt-1 flex items-center space-x-1.5 px-3 py-0.5 bg-gradient-to-r from-orange-600 to-amber-500 rounded-full text-xs font-black tracking-wider uppercase shadow-lg shadow-orange-500/50 animate-bounce">
              <Flame className="w-3.5 h-3.5 text-yellow-200 fill-yellow-200" />
              <span>連擊 COMBO x{combo}!</span>
            </div>
          )}
        </div>

        {/* Right: Score, Board Coin Count, Settings */}
        <div className="flex items-center space-x-2 pointer-events-auto order-2 md:order-3">
          {/* Score Counter */}
          <div className="flex items-center space-x-2 px-3.5 py-1.5 rounded-xl bg-slate-900/70 border border-sky-500/40 glow-cyan text-sky-200 backdrop-blur-md">
            <Trophy className="w-5 h-5 text-sky-400" />
            <div>
              <div className="text-[9px] uppercase font-bold tracking-wider opacity-70">
                推落得分
              </div>
              <div className="text-lg md:text-xl font-black tabular-nums">
                {score}
              </div>
            </div>
            {maxCombo > 1 && (
              <div className="ml-2 pl-2 border-l border-slate-700/80 text-[10px] text-amber-300 font-bold">
                <div>最高連擊</div>
                <div className="font-mono text-sm">{maxCombo}x</div>
              </div>
            )}
          </div>

          {/* Board Coin Counter */}
          <div className="hidden sm:flex items-center space-x-1.5 px-2.5 py-1.5 rounded-xl bg-slate-900/60 border border-slate-700/50 text-slate-300 text-xs backdrop-blur-md">
            <Layers className="w-4 h-4 text-amber-400" />
            <span className="font-mono">{activeBoardCoinCount} 枚</span>
          </div>

          {/* Sound Toggle */}
          <button
            onClick={() => useGameStore.getState().toggleMute()}
            className="p-2.5 rounded-xl bg-slate-900/80 border border-slate-700/60 hover:bg-slate-800 text-slate-200 hover:text-white transition-colors cursor-pointer"
            title={isMuted ? '取消靜音' : '靜音'}
          >
            {isMuted ? <VolumeX className="w-5 h-5 text-red-400" /> : <Volume2 className="w-5 h-5 text-emerald-400" />}
          </button>

          {/* Help Modal Toggle */}
          <button
            onClick={() => setShowHelp((prev) => !prev)}
            className="p-2.5 rounded-xl bg-slate-900/80 border border-slate-700/60 hover:bg-slate-800 text-slate-200 hover:text-white transition-colors cursor-pointer"
            title="遊戲說明"
          >
            <HelpCircle className="w-5 h-5" />
          </button>

          {/* Reset Board */}
          <button
            onClick={handleResetGame}
            className="p-2.5 rounded-xl bg-slate-900/80 border border-slate-700/60 hover:bg-slate-800 text-slate-400 hover:text-red-400 transition-colors cursor-pointer"
            title="重置機台與分數"
          >
            <RotateCcw className="w-5 h-5" />
          </button>
        </div>
      </header>

      {/* ============================================================
          4. BOTTOM ARCADE CONTROLLER BAR
         ============================================================ */}
      <footer className="absolute bottom-0 left-0 right-0 z-30 p-4 md:p-6 flex flex-col items-center pointer-events-none">
        <div className="flex flex-wrap items-center justify-center gap-3 md:gap-4 pointer-events-auto bg-slate-950/80 backdrop-blur-xl border border-slate-800/80 px-5 py-3.5 rounded-3xl shadow-2xl">
          {/* Shake Machine Button */}
          <button
            onClick={handleTriggerShake}
            disabled={!canShake}
            className={`px-4 py-3 rounded-2xl font-black text-sm tracking-wider uppercase transition-all flex items-center space-x-2 cursor-pointer ${
              canShake
                ? 'bg-gradient-to-r from-red-600 to-rose-500 text-white shadow-lg shadow-rose-600/40 hover:scale-105 active:scale-95'
                : 'bg-slate-800 text-slate-500 cursor-not-allowed border border-slate-700/40'
            }`}
            title="搖晃機台 (熱鍵 S)"
          >
            <Zap className="w-4 h-4" />
            <span>{canShake ? '搖晃機台 [S]' : `冷卻中 (${shakeCooldown}s)`}</span>
          </button>

          {/* BIG JUICY DROP COIN BUTTON */}
          <button
            onClick={handleDropCoin}
            disabled={coins <= 0}
            className={`px-8 md:px-12 py-3.5 md:py-4 rounded-2xl font-black text-lg md:text-xl tracking-wider uppercase transition-all transform active:scale-95 cursor-pointer flex items-center space-x-3 ${
              coins > 0
                ? 'bg-gradient-to-r from-amber-400 via-yellow-400 to-amber-500 text-slate-950 glow-gold hover:brightness-110 shadow-2xl'
                : 'bg-slate-800 text-slate-500 cursor-not-allowed border border-slate-700'
            }`}
          >
            <Coins className="w-6 h-6 fill-current animate-bounce" />
            <span>投幣 [空白鍵]</span>
          </button>

          {/* Continuous Auto-Drop Toggle */}
          <button
            onClick={() => setAutoDrop((prev) => !prev)}
            disabled={coins <= 0 && !autoDrop}
            className={`px-4 py-3 rounded-2xl font-black text-sm tracking-wider uppercase transition-all flex items-center space-x-2 cursor-pointer ${
              autoDrop
                ? 'bg-gradient-to-r from-pink-500 to-purple-600 text-white glow-fever animate-pulse'
                : 'bg-slate-800/80 hover:bg-slate-800 border border-slate-700/60 text-slate-300'
            }`}
            title="自動連投 (熱鍵 A)"
          >
            {autoDrop ? <Square className="w-4 h-4 fill-current" /> : <Play className="w-4 h-4 fill-current" />}
            <span>{autoDrop ? '停止自動 [A]' : '自動連投 [A]'}</span>
          </button>
        </div>

        {/* Bottom Subtitle / Prompt */}
        <div className="mt-2 text-[11px] text-slate-400/80 font-medium tracking-wide">
          💡 提示：透明側板與安全空氣牆已就位，硬幣全面集中推向前方！拖曳滑鼠可微調街機視角。
        </div>
      </footer>

      {/* ============================================================
          5. HELP / INFO MODAL
         ============================================================ */}
      {showHelp && (
        <div className="absolute inset-0 z-50 bg-black/75 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="bg-slate-900 border border-slate-700 rounded-3xl p-6 md:p-8 max-w-md w-full shadow-2xl text-left space-y-4">
            <div className="flex items-center justify-between border-b border-slate-800 pb-3">
              <h2 className="text-xl font-black text-amber-400 flex items-center space-x-2">
                <Sparkles className="w-5 h-5" />
                <span>廉價多巴胺推幣機 遊戲說明</span>
              </h2>
              <button
                onClick={() => setShowHelp(false)}
                className="text-slate-400 hover:text-white font-black text-lg p-1 cursor-pointer"
              >
                ✕
              </button>
            </div>

            <div className="space-y-3 text-sm text-slate-300 leading-relaxed">
              <p>
                🪙 <strong className="text-amber-300">導引前進、全歸玩家</strong>：
                機台兩側配有壓克力側板，硬幣將被推板集中往前推擠，掉入出幣槽通通歸玩家所有並回補籌碼！
              </p>
              <p>
                🛡️ <strong className="text-emerald-300">防脫軌空氣牆保護</strong>：
                機台設有全方位空氣牆，防止硬幣在劇烈推擠時噴飛出鏡頭外或卡入後方機械。
              </p>
              <p>
                🎯 <strong className="text-sky-300">移動投幣口</strong>：
                上方的投幣口會自動左右平移，看準推板縮回的黃金時機按下「空白鍵」或點擊投幣。
              </p>
              <p>
                ⚡ <strong className="text-pink-400">FEVER 狂熱雙倍模式</strong>：
                每推下一枚硬幣蓄力 +5%，滿 100% 進入 10 秒狂熱時間，分數與金幣返還翻倍！
              </p>
              <p>
                🚨 <strong className="text-rose-400">機台搖晃 (Tilt)</strong>：
                遇到硬幣卡在邊緣搖搖欲墜時，按「S」鍵或點擊「搖晃機台」產生震盪推擠！
              </p>
              <p>
                🎁 <strong className="text-emerald-400">破產救濟</strong>：
                硬幣歸零時，點擊「免費補幣」立刻獲贈 50 枚硬幣，讓你無限暢玩！
              </p>
            </div>

            <div className="pt-2">
              <button
                onClick={() => setShowHelp(false)}
                className="w-full py-3 bg-gradient-to-r from-amber-400 to-amber-500 text-slate-950 font-black rounded-xl hover:brightness-110 cursor-pointer shadow-lg shadow-amber-500/30"
              >
                開始爽推！
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
