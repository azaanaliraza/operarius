import React, { useState, useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { RefreshCcw, Wifi, AlertTriangle, HardDrive } from 'lucide-react';
import { useOnboardingStore } from '../../lib/onboardingStore';
import panther from '../../assets/panther.png';

interface DownloadProgress {
  progress: number;
  speed_mb: number;
  downloaded_gb: number;
}

const TELEMETRY_LOGS = [
  "INFRA: [SECURE] SYNCING CDN NODE...",
  "STORAGE: [IO] ALLOCATING TENSORS...",
  "MANIFEST: [LFS] PULLING POINTERS...",
  "ACCEL: [KERN] OPTIMIZING METAL...",
  "TCP: [TUN] SOCKET_STABILIZE...",
  "STREAM: [NET] FETCHING BLOCKS...",
  "MEM: [UNM] UNIFIED_CACHE_ALLOC...",
  "RESUME: [SYNC] ALIGNING RANGE_HEADER...",
  "RESUME: [FETCH] BUFFER_RECOVERY_ACTIVE...",
  "NETWORK: [AUTO] HEARTBEAT_LOCK_ON..."
];

const DownloadScreen: React.FC<{ onComplete: () => void }> = ({ onComplete }) => {
  const { modelRepo, modelFile } = useOnboardingStore();
  const setupStarted = useRef(false);
  
  const targetProgress = useRef(0);
  const targetSpeed = useRef(0);
  const targetDownloaded = useRef(0);
  
  const [displayMetrics, setDisplayMetrics] = useState({ percent: 0, speed: 0, downloaded: 0 });
  const [currentLog, setCurrentLog] = useState(TELEMETRY_LOGS[0]);
  const [errorStatus, setErrorStatus] = useState<string | null>(null);
  const [isStationed, setIsStationed] = useState(false);
  const logIndex = useRef(0);

  useEffect(() => {
    const setupHandlers = async () => {
      // 1. Initial Discovery: Check if model exists before starting stream
      if (modelFile) {
        const exists = await invoke<boolean>('check_model_exists', { filename: modelFile });
        if (exists) {
          setIsStationed(true);
          targetProgress.current = 100;
          setCurrentLog("NODE: [MATCH] LOCAL CACHE IDENTIFIED");
        }
      }

      const unlistenProgress = await listen<DownloadProgress>('download-progress', (event) => {
        targetProgress.current = Math.max(targetProgress.current, event.payload.progress);
        targetDownloaded.current = Math.max(targetDownloaded.current, event.payload.downloaded_gb);
        targetSpeed.current = event.payload.speed_mb;
        setErrorStatus(null);
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
        percent: Math.min(100, prev.percent + (targetProgress.current - prev.percent) * (isStationed ? 0.05 : 0.4)),
        speed: prev.speed + (targetSpeed.current - prev.speed) * 0.2, 
        downloaded: prev.downloaded + (targetDownloaded.current - prev.downloaded) * 0.4
      }));
      animId = requestAnimationFrame(smooth);
    };
    smooth();

    const logInterval = setInterval(() => {
      if (!errorStatus && !isStationed) {
        logIndex.current = (logIndex.current + 1) % TELEMETRY_LOGS.length;
        setCurrentLog(TELEMETRY_LOGS[logIndex.current]);
      }
    }, 2000);

    const startProvisioning = async () => {
      if (!setupStarted.current && modelRepo && modelFile) {
        setupStarted.current = true;
        try {
          await invoke('complete_local_setup', { modelRepo, modelFile });
          setCurrentLog("NODE: [WAIT] PREPARING RAG AGENT...");
          await invoke('create_default_rag_agent', { modelFile });

          targetProgress.current = 100;
          setCurrentLog("NODE: [READY] SYSTEM READY");
          setTimeout(onComplete, isStationed ? 1000 : 1500);
        } catch (err) {
          console.error(err);
          setErrorStatus(String(err));
          setCurrentLog("NETWORK: [HALT] CONNECTION_LOST");
          setupStarted.current = false;
        }
      }
    };
    startProvisioning();

    return () => {
      cancelAnimationFrame(animId);
      clearInterval(logInterval);
    };
  }, [modelRepo, modelFile, onComplete, errorStatus, isStationed]);

  return (
    <div className="h-screen w-screen bg-[#F8F9FA] flex flex-col items-center justify-center p-4 font-sans selection:bg-black selection:text-white overflow-hidden relative">
      
      <div className="w-full max-w-[440px] text-center relative z-10 animate-in fade-in zoom-in-95 duration-1000">
        
        <div className="relative w-16 h-16 mx-auto mb-6">
          <div className="absolute inset-0 bg-black rounded-3xl rotate-12 opacity-5 scale-90 -z-10 animate-pulse"></div>
          <img src={panther} className="w-full h-full object-contain brightness-0 grayscale" />
        </div>

        <h2 className="text-xl font-black text-[#1C1C1E] tracking-tighter uppercase mb-4 italic">
          {isStationed ? 'Syncing Station' : 'Downloading'}
        </h2>
        
        {/* PULSE LOG FEED */}
        <div className="h-8 flex items-center justify-center mb-8 relative">
           <div 
             key={currentLog} 
             className={`font-mono text-[9px] font-black uppercase tracking-[0.4em] animate-[pulseFade_2s_ease-in-out_infinite] transition-colors
               ${errorStatus ? 'text-rose-500' : isStationed ? 'text-emerald-500' : 'text-black/40'}`}
           >
             {errorStatus ? `ERROR: ${errorStatus.toUpperCase()}` : currentLog}
           </div>
        </div>

        {/* Global Metric Container */}
        <div className={`bg-white border rounded-[2.5rem] p-10 shadow-2xl relative transition-colors duration-500
          ${errorStatus ? 'border-rose-100' : isStationed ? 'border-emerald-100 shadow-emerald-500/5' : 'border-gray-100'}`}>
          
          <div className="flex justify-between items-end mb-8 relative">
            <div className="text-left w-28">
              <div className="text-[7px] font-black text-gray-300 uppercase tracking-widest mb-1.5 leading-none">Bandwidth</div>
              <div className={`font-mono text-base font-black tabular-nums tracking-tighter transition-colors ${errorStatus ? 'text-rose-400' : isStationed ? 'text-emerald-500' : 'text-[#1C1C1E]'}`}>
                {isStationed ? 'LOCAL' : `${displayMetrics.speed.toFixed(1)}`} <span className="text-[10px] opacity-20 ml-1">{isStationed ? 'LAN' : 'MB/s'}</span>
              </div>
            </div>
            
            <div className="text-right w-28">
              <div className="text-[7px] font-black text-gray-300 uppercase tracking-widest mb-1.5 leading-none">Progress</div>
              <div className={`font-mono text-base font-black tabular-nums tracking-tighter italic transition-colors ${errorStatus ? 'text-rose-400' : isStationed ? 'text-emerald-500' : 'text-[#1C1C1E]'}`}>
                {displayMetrics.percent.toFixed(1)}%
              </div>
            </div>
          </div>

          {/* Kinetic Progress Bar */}
          <div className="h-3 w-full bg-gray-50 rounded-full overflow-hidden mb-8 border border-gray-100 p-0.5 relative shadow-inner">
            <div 
              className={`h-full rounded-full relative transition-all duration-75 shadow-lg
                ${errorStatus ? 'bg-rose-500 shadow-rose-200' : isStationed ? 'bg-emerald-500 shadow-emerald-200' : 'bg-black shadow-black/10'}`}
              style={{ width: `${displayMetrics.percent}%` }}
            >
               <div className="absolute top-0 right-0 bottom-0 w-32 bg-gradient-to-r from-transparent via-white/10 to-transparent animate-[shimmer_2s_infinite]"></div>
            </div>
          </div>

          {/* Multi-Status Footer */}
          <div className="flex justify-between items-center text-[9px] font-black uppercase tracking-[0.2em] text-[#9CA3AF] tabular-nums">
             <div className="flex items-center gap-2">
                {errorStatus ? (
                  <>
                    <AlertTriangle className="w-3.5 h-3.5 text-rose-500" />
                    <span className="text-rose-500">Resume Blocked</span>
                  </>
                ) : isStationed ? (
                  <>
                    <HardDrive className="w-3.5 h-3.5 text-emerald-500" />
                    <span className="text-emerald-500">Direct Link Active</span>
                  </>
                ) : (
                  <>
                    <div className="w-1.5 h-1.5 bg-emerald-500 rounded-full animate-pulse"></div>
                    <span className="text-emerald-500/80">Stream Active</span>
                  </>
                )}
             </div>
             <div className={`px-4 py-1.5 rounded-full border font-mono shadow-sm transition-all
               ${errorStatus ? 'bg-rose-50 border-rose-100 text-rose-400' : isStationed ? 'bg-emerald-50 border-emerald-100 text-emerald-600' : 'bg-gray-50 border-gray-100 text-black'}`}>
               {displayMetrics.downloaded.toFixed(2)} <span className="opacity-20 ml-0.5">GB</span>
             </div>
          </div>
        </div>

        {/* Action Recovery Area */}
        {errorStatus ? (
          <button 
            onClick={() => window.location.reload()}
            className="mt-10 bg-black text-white px-8 py-3.5 rounded-2xl flex items-center gap-3 mx-auto font-black text-[10px] uppercase tracking-widest animate-in slide-in-from-bottom-4 transition-all hover:scale-105 active:scale-95"
          >
            <RefreshCcw className="w-3.5 h-3.5" /> RE-IGNITE DOWNLOAD
          </button>
        ) : (
          <div className="mt-14 flex items-center justify-center gap-4 opacity-30">
             <Wifi className={`w-3 h-3 ${isStationed ? 'text-emerald-500' : 'text-emerald-500'}`} />
             <span className="text-[7.5px] font-black uppercase tracking-[0.6em] text-gray-400 italic">
               {isStationed ? 'Hardware-Accelerated Vault' : 'Operarius Persistence Node'}
             </span>
          </div>
        )}

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
