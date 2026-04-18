import React, { useState, useEffect } from 'react';
import { Check, Download, Shield } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { useOnboardingStore } from '../../lib/onboardingStore';
import panther from '../../assets/panther.png';

interface HardwareInfo {
  cpu_brand: string;
  cpu_cores: number;
  ram_gb: number;
  storage_free_gb: u64;
  recommended_model: string;
}

const ModelPicker: React.FC<{ onContinue: () => void }> = ({ onContinue }) => {
  const [hw, setHw] = useState<HardwareInfo | null>(null);
  const { setSelectedModel } = useOnboardingStore();
  
  const models = [
    { 
      name: "Phi-4 Mini Q4_K_M", 
      size: "0.8GB", 
      speed: "22T/S", 
      tag: "SPEED", 
      repo: "unsloth/Phi-4-mini-instruct-GGUF", 
      file: "Phi-4-mini-instruct-Q4_K_M.gguf" 
    },
    { 
      name: "Llama 3.2 3B Q4_K_M", 
      size: "2.0GB", 
      speed: "16T/S", 
      tag: "COMPACT", 
      repo: "unsloth/Llama-3.2-3B-Instruct-GGUF", 
      file: "Llama-3.2-3B-Instruct-Q4_K_M.gguf" 
    },
    { 
      name: "Hermes 3 8B Q4_K_M", 
      size: "4.9GB", 
      speed: "09T/S", 
      tag: "INTELLIGENCE", 
      repo: "NousResearch/Hermes-3-Llama-3.1-8B-GGUF", 
      file: "Hermes-3-Llama-3.1-8B-Q4_K_M.gguf" 
    },
    { 
      name: "Qwen 2.5 7B Q4_K_M", 
      size: "4.7GB", 
      speed: "11T/S", 
      tag: "BALANCED", 
      repo: "unsloth/Qwen2.5-7B-Instruct-GGUF", 
      file: "Qwen2.5-7B-Instruct-Q4_K_M.gguf" 
    },
    { 
      name: "Gemma 4 26B-A4B Q4_K_M", 
      size: "4.2GB", 
      speed: "07T/S", 
      tag: "PRO", 
      repo: "unsloth/gemma-4-26B-A4B-it-GGUF", 
      file: "gemma-4-26B-A4B-it-UD-Q4_K_M.gguf" 
    }
  ];

  const [localSelected, setLocalSelected] = useState(models[1].name);

  useEffect(() => {
    invoke<HardwareInfo>('scan_hardware')
      .then(info => {
        setHw(info);
        if (info.ram_gb <= 8) setLocalSelected(models[0].name);
      })
      .catch(console.error);
  }, []);

  const handleContinue = () => {
    const model = models.find(m => m.name === localSelected);
    if (model) {
      setSelectedModel(model.name, model.repo, model.file);
      onContinue();
    }
  };

  return (
    <div className="min-h-screen bg-[#F8F9FA] flex flex-col items-center py-10 px-4 font-sans overflow-y-auto selection:bg-black selection:text-white no-scrollbar">
      <div className="w-full max-w-[480px] animate-in fade-in duration-700">
        
        <div className="text-center mb-6">
          <img src={panther} alt="Operarius" className="w-16 h-16 mx-auto mb-4 object-contain brightness-0" />
          <h1 className="text-xl font-black text-[#1C1C1E] tracking-tighter italic uppercase">Engine Fleet</h1>
          <p className="text-gray-400 text-[8px] font-black uppercase tracking-[0.4em] mt-1 opacity-50 italic">Verified Local Weights</p>
        </div>

        <div className="grid grid-cols-4 gap-2 mb-6">
          <MiniStat label="SILICON" value={hw ? hw.cpu_brand.split(' ')[0].replace("Apple", "M-") : "..."} />
          <MiniStat label="MEMORY" value={hw ? `${hw.ram_gb}G` : "..."} />
          <MiniStat label="STG FREE" value={hw ? `${hw.storage_free_gb}G` : "..."} />
          <MiniStat label="CORES" value={hw ? `${hw.cpu_cores}` : "..."} />
        </div>

        <div className="space-y-2 mb-10">
          {models.map((model) => (
            <div 
              key={model.name}
              onClick={() => setLocalSelected(model.name)}
              className={`bg-white border-[2px] rounded-2xl p-4 flex items-center justify-between cursor-pointer transition-all duration-300 ${
                localSelected === model.name ? 'border-black shadow-lg scale-[1.01]' : 'border-gray-50 opacity-60 hover:opacity-100'
              }`}
            >
              <div className="flex items-center gap-4 flex-1 min-w-0">
                <div className={`w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0 ${
                  localSelected === model.name ? 'bg-black' : 'bg-gray-50'
                }`}>
                  <img src={panther} className={`w-4 h-4 object-contain ${localSelected === model.name ? 'invert' : 'brightness-0 opacity-20'}`} />
                </div>
                <div className="min-w-0 flex-1">
                  <div className="font-black text-[11px] text-[#1C1C1E] tracking-tighter truncate uppercase">{model.name}</div>
                  <div className="text-gray-400 font-mono text-[8px] font-black tracking-widest uppercase">
                    {model.size} / {model.speed}
                  </div>
                </div>
              </div>
              <div className="flex items-center gap-3">
                <span className={`text-[7px] font-black px-2 py-0.5 rounded-full border uppercase tracking-widest hidden sm:inline ${
                  localSelected === model.name ? 'bg-black text-white border-black' : 'bg-gray-50 border-gray-100 text-gray-300'
                }`}>
                  {model.tag}
                </span>
                <div className={`w-4 h-4 rounded-full border flex items-center justify-center ${
                  localSelected === model.name ? 'bg-black border-black' : 'border-gray-100'
                }`}>
                  {localSelected === model.name && <Check className="w-2.5 h-2.5 text-white" />}
                </div>
              </div>
            </div>
          ))}
        </div>

        <div className="flex justify-center">
          <button 
            onClick={handleContinue}
            className="w-full bg-[#1C1C1E] text-white py-4 rounded-2xl font-black flex items-center justify-center gap-3 hover:bg-black transition-all shadow-xl active:scale-98 text-[10px] uppercase tracking-[0.3em]"
          >
            INITIALIZE ENGINE <Download className="w-4 h-4" />
          </button>
        </div>

        <div className="mt-8 flex items-center justify-center gap-2 opacity-10">
           <Shield className="w-2.5 h-2.5" />
           <span className="text-[7px] font-black uppercase tracking-[0.4em]">Hardware-Encrypted Local Cache</span>
        </div>
      </div>
    </div>
  );
};

const MiniStat: React.FC<{ label: string, value: string }> = ({ label, value }) => (
  <div className="bg-white rounded-xl p-3 border border-gray-50 shadow-sm flex flex-col justify-center text-center py-4">
    <span className="text-[7px] font-black text-gray-300 uppercase tracking-widest mb-1 leading-none">{label}</span>
    <span className="font-mono text-[10px] font-black text-[#1C1C1E] tracking-tighter">{value}</span>
  </div>
);

export default ModelPicker;
