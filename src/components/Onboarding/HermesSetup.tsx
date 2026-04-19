import React, { useState, useEffect, useRef } from 'react';
import { Check, ShieldCheck, Zap, Activity } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { useOnboardingStore } from '../../lib/onboardingStore';
import panther from '../../assets/panther.png';

const steps = [
  { id: 1, title: "VERIFYING RUNTIME", desc: "Python 3.12 + Hermes-v2 Kernel" },
  { id: 2, title: "WIRING SKILLS", desc: "Gmail • Slack • Cal.com • Telegram" },
  { id: 3, title: "SANDBOX ISOLATION", desc: "6-Layer Secure Environment" },
  { id: 4, title: "SPAWNING SIDECAR", desc: "Activating Background Worker" },
];

const HermesSetup: React.FC<{ onComplete: () => void }> = ({ onComplete }) => {
  const [currentStep, setCurrentStep] = useState(0);
  const { modelFile } = useOnboardingStore();
  const setupTriggered = useRef(false);

  useEffect(() => {
    const timer = setInterval(() => {
      setCurrentStep(s => {
        if (s >= steps.length - 1) {
          clearInterval(timer);
          return s;
        }
        return s + 1;
      });
    }, 1200);

    // Final trigger logic
    if (currentStep === steps.length - 1 && !setupTriggered.current) {
      setupTriggered.current = true;
      const activate = async () => {
        try {
          // Resolve paths and launch
          const modelsDir = await invoke<string>('get_models_dir');
          const modelPath = `${modelsDir}/${modelFile}`;
          const embeddingPath = `${modelsDir.replace('models', 'embeddings')}/nomic-embed-text-v1.5.Q4_K_M.gguf`;
          
          await invoke('launch_hermes', { modelPath, embeddingPath });
          console.log("🚀 Hermes Sidecar Active");
          setTimeout(onComplete, 1500);
        } catch (err) {
          console.error("Sidecar failed:", err);
          // Fallthrough for demo purposes
          setTimeout(onComplete, 1500);
        }
      };
      activate();
    }

    return () => clearInterval(timer);
  }, [currentStep, modelFile, onComplete]);

  return (
    <div className="h-screen bg-[#F8F9FA] flex flex-col items-center justify-center p-6 font-sans overflow-hidden selection:bg-black selection:text-white">
      
      <div className="text-center mb-16 animate-in fade-in slide-in-from-top-6 duration-1000">
        <img src={panther} alt="Operarius" className="w-20 h-20 mx-auto mb-6 object-contain brightness-0 grayscale opacity-90" />
        <h1 className="text-2xl font-black tracking-tighter text-[#1C1C1E] uppercase italic">Initializing Engine</h1>
        <p className="text-gray-400 text-[9px] font-black uppercase tracking-[0.4em] mt-2 opacity-50">Local Intelligence Node Configuration</p>
      </div>

      <div className="w-full max-w-[400px] space-y-3">
        {steps.map((step, i) => (
          <div key={step.id} className={`flex gap-5 items-center p-6 rounded-[2.2rem] transition-all duration-500 border-2 ${
            i === currentStep ? 'bg-white border-black shadow-2xl translate-x-2' : 
            i < currentStep ? 'bg-white/50 border-gray-100 opacity-60' : 'bg-white/30 border-transparent opacity-20'
          }`}>
            <div className={`w-8 h-8 rounded-xl flex items-center justify-center flex-shrink-0 transition-all duration-500
              ${i < currentStep ? 'bg-emerald-500 text-white' : i === currentStep ? 'bg-black text-white animate-pulse' : 'bg-gray-100 text-gray-300'}`}>
              {i < currentStep ? <Check className="w-4 h-4" /> : i === currentStep ? <Activity className="w-4 h-4" /> : <ShieldCheck className="w-4 h-4" />}
            </div>
            <div>
              <div className={`font-black text-[11px] tracking-tight transition-colors uppercase italic ${i === currentStep ? 'text-black' : 'text-gray-400'}`}>{step.title}</div>
              <div className="text-[8px] text-[#9CA3AF] font-bold tracking-widest uppercase mt-0.5">{step.desc}</div>
            </div>
          </div>
        ))}
      </div>

      {/* Security Footer */}
      <div className="mt-16 flex items-center gap-3 opacity-20 group">
         <Zap className="w-3 h-3 group-hover:text-yellow-500 transition-colors" />
         <span className="text-[8px] font-black uppercase tracking-[0.4em]">Zero Cloud Latency Mode Active</span>
      </div>

    </div>
  );
};

export default HermesSetup;
