import React, { useState, useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { useOnboardingStore } from '../../lib/onboardingStore';
import panther from '../../assets/panther.png';

interface DownloadProgress {
  progress: number;
  speed_mb: number;
  downloaded_gb: number;
}

const TELEMETRY_LOGS = [
  "INFRA: [SECURE] LINKING TO CDN_CORE...",
  "STORAGE: [IO] ALLOCATING TENSOR BUFFER...",
  "MANIFEST: [LFS] RESOLVING ASSET POINTERS...",
  "ACCEL: [KERN] SYNCING METAL SHADERS...",
  "STREAM: [NET] FETCHING TENSOR BLOCK...",
  "MEMORY: [UNM] OPTIMIZING UNIFIED CACHE...",
  "PERSIST: [SQL] GENERATING RAG SCHEMA...",
  "KEYRING: [OSX] SECURING LOCAL TOKEN...",
  "HARDWARE: [M-CHIP] AUDIT COMPLETE...",
  "VECTOR: [VEC] PREPARING NOMIC SPACE...",
  "CRC: [INTEG] VERIFYING WEIGHTS...",
  "SYNC: [NODE] PROTOCOL V2 ACTIVE...",
  "TCP: [TUN] SOCKET_STABILIZE..."
];

const DownloadScreen: React.FC<{ onComplete: () => void }> = ({ onComplete }) => {
  const { modelRepo, modelFile } = useOnboardingStore();
  const setupStarted = useRef(false);
  
  const targetProgress = useRef(0);
  const targetSpeed = useRef(0);
  const targetDownloaded = useRef(0);
  
  const [displayMetrics, setDisplayMetrics] = useState({ percent: 0, speed: 0, downloaded: 0 });
  const [currentLog, setCurrentLog] = useState(TELEMETRY_LOGS[0]);
  const logIndex = useRef(0);

  useEffect(() => {
    const setupHandlers = async () => {
      const unlistenProgress = await listen<DownloadProgress>('download-progress', (event) => {
        targetProgress.current = Math.max(targetProgress.current, event.payload.progress);
        targetDownloaded.current = Math.max(targetDownloaded.current, event.payload.downloaded_gb);
        targetSpeed.current = event.payload.speed_mb;
      });
      const unlistenComplete = await listen('download-complete', () => {
         targetProgress.current = 100;
         setCurrentLog("NODE: [READY] DEPLOYMENT SUCCESS");
      });
      return () => { unlistenProgress(); unlistenComplete(); };
    };
    setupHandlers();

    let animId: number;
    const smooth = () => {
      setDisplayMetrics(prev => ({
        percent: prev.percent + (targetProgress.current - prev.percent) * 0.4,
        speed: prev.speed + (targetSpeed.current - prev.speed) * 0.2, 
        downloaded: prev.downloaded + (targetDownloaded.current - prev.downloaded) * 0.4
      }));
      animId = requestAnimationFrame(smooth);
    };
    smooth();

    // SEQUENTIAL PULSE: One log at a time with fade
    const logInterval = setInterval(() => {
      logIndex.current = (logIndex.current + 1) % TELEMETRY_LOGS.length;
      setCurrentLog(TELEMETRY_LOGS[logIndex.current]);
    }, 2000); // 2 second pulse cycle

    if (!setupStarted.current && modelRepo && modelFile) {
      setupStarted.current = true;
      invoke('complete_local_setup', { modelRepo, modelFile })
        .then(() => {
          targetProgress.current = 100;
          setTimeout(onComplete, 2000);
        })
        .catch(err => setCurrentLog(`CRITICAL: [FAIL] ${err}`));
    }

    return () => {
      cancelAnimationFrame(animId);
      clearInterval(logInterval);
    };
  }, [modelRepo, modelFile, onComplete]);

  return (
    <div className="h-screen w-screen bg-[#F8F9FA] flex flex-col items-center justify-center p-4 font-sans selection:bg-black selection:text-white overflow-hidden relative">
      
      <div className="w-full max-w-[440px] text-center relative z-10 animate-in fade-in zoom-in-95 duration-1000">
        
        <div className="relative w-16 h-16 mx-auto mb-6">
          <div className="absolute inset-0 bg-black rounded-3xl rotate-12 opacity-5 scale-90 -z-10 animate-pulse"></div>
          <img src={panther} className="w-full h-full object-contain brightness-0 grayscale" />
        </div>

        <h2 className="text-xl font-black text-[#1C1C1E] tracking-tighter uppercase mb-4">Downloading</h2>
        
        {/* PULSE LOG FEED */}
        <div className="h-8 flex items-center justify-center mb-8">
           <div 
             key={currentLog} 
             className="font-mono text-[9px] font-black text-black/40 uppercase tracking-[0.4em] animate-[pulseFade_2s_ease-in-out_infinite]"
           >
             {currentLog}
           </div>
        </div>

        {/* Global Metric Container */}
        <div className="bg-white border border-gray-100 rounded-[2.5rem] p-10 shadow-2xl relative overflow-hidden backdrop-blur-md">
          
          <div className="flex justify-between items-end mb-8">
            <div className="text-left w-28">
              <div className="text-[7px] font-black text-gray-300 uppercase tracking-[0.3em] mb-1.5 leading-none">Bandwidth</div>
              <div className="font-mono text-base font-black text-[#1C1C1E] tabular-nums tracking-tighter">
                {displayMetrics.speed.toFixed(1)} <span className="text-[10px] opacity-20 ml-1">MB/s</span>
              </div>
            </div>
            
            <div className="text-right w-28">
              <div className="text-[7px] font-black text-gray-300 uppercase tracking-[0.3em] mb-1.5 leading-none">Progress</div>
              <div className="font-mono text-base font-black text-[#1C1C1E] tabular-nums tracking-tighter italic">
                {displayMetrics.percent.toFixed(1)}%
              </div>
            </div>
          </div>

          <div className="h-3 w-full bg-gray-50 rounded-full overflow-hidden mb-8 border border-gray-100 p-0.5 relative shadow-inner">
            <div 
              className="h-full bg-black rounded-full relative shadow-[0_0_20px_rgba(0,0,0,0.1)] transition-all duration-75"
              style={{ width: `${displayMetrics.percent}%` }}
            >
               <div className="absolute top-0 right-0 bottom-0 w-32 bg-gradient-to-r from-transparent via-white/10 to-transparent animate-[shimmer_2s_infinite]"></div>
            </div>
          </div>

          <div className="flex justify-between items-center text-[9px] font-black uppercase tracking-[0.2em] text-[#9CA3AF] tabular-nums">
             <div className="flex items-center gap-2">
                <div className="w-1.5 h-1.5 bg-emerald-500 rounded-full animate-pulse"></div>
                <span>Stream Sync</span>
             </div>
             <div className="text-black bg-gray-50 px-4 py-1.5 rounded-full border border-gray-100 font-mono shadow-sm">
               {displayMetrics.downloaded.toFixed(2)} <span className="opacity-20 ml-0.5">GB</span>
             </div>
          </div>
        </div>

        <div className="mt-14 flex items-center justify-center gap-4 opacity-30">
           <div className="h-[0.5px] w-12 bg-gray-400"></div>
           <span className="text-[7.5px] font-black uppercase tracking-[0.6em] text-gray-400 italic">Operarius Local Node</span>
           <div className="h-[0.5px] w-12 bg-gray-400"></div>
        </div>

      </div>

      <style dangerouslySetInnerHTML={{ __html: `
        @keyframes shimmer {
          0% { transform: translateX(-150%); }
          100% { transform: translateX(150%); }
        }
        @keyframes pulseFade {
          0% { opacity: 0; transform: translateY(4px); }
          20% { opacity: 1; transform: translateY(0); }
          80% { opacity: 1; transform: translateY(0); }
          100% { opacity: 0; transform: translateY(-4px); }
        }
      `}} />
    </div>
  );
};

export default DownloadScreen;
