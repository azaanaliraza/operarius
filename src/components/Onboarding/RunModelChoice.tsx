import React from 'react';
import { Cpu, Cloud, Zap } from 'lucide-react';

const RunModelChoice: React.FC<{ onSelectLocal: () => void }> = ({ onSelectLocal }) => {
  return (
    <div className="h-screen w-screen bg-[#F8F9FA] flex flex-col items-center justify-center p-8 overflow-hidden animate-in fade-in duration-700 selection:bg-black selection:text-white">
      <div className="w-full max-w-[420px] text-center">
        
        <div className="mb-10">
          <div className="text-[10px] font-bold text-gray-400 uppercase tracking-[0.25em] mb-3 opacity-50">Local or Cloud</div>
          <h1 className="text-2xl font-black tracking-tighter text-[#1C1C1E]">DEPLOYMENT PATH</h1>
        </div>

        <div className="grid grid-cols-1 gap-3 mb-12">
          <ChoiceCard
            icon={<Cpu className="w-5 h-5" />}
            title="Sovereign Local"
            description="Run everything on your Mac. 100% private, offline capable."
            onClick={onSelectLocal}
            recommended
          />
          <ChoiceCard
            icon={<Cloud className="w-5 h-5" />}
            title="Hybrid Cloud"
            description="Google Vertex & Anthropic. Higher speed, less battery impact."
            onClick={() => {}}
            disabled
          />
        </div>

        <p className="text-[9px] text-gray-400 font-bold uppercase tracking-widest px-10 leading-relaxed opacity-40">
          You can toggle your primary engine at any time from settings.
        </p>
      </div>
    </div>
  );
};

const ChoiceCard: React.FC<{ 
  icon: React.ReactNode, 
  title: string, 
  description: string, 
  onClick: () => void,
  recommended?: boolean,
  disabled?: boolean
}> = ({ icon, title, description, onClick, recommended, disabled }) => (
  <button
    onClick={onClick}
    disabled={disabled}
    className={`group relative w-full p-6 bg-white border-2 rounded-[2rem] text-left transition-all duration-300 ${
      disabled ? 'opacity-40 grayscale cursor-not-allowed' : 'hover:border-black cursor-pointer shadow-sm hover:shadow-xl'
    } ${recommended ? 'border-gray-50' : 'border-gray-50'}`}
  >
    <div className="flex items-center gap-5">
      <div className={`w-12 h-12 rounded-2xl flex items-center justify-center transition-colors ${
        disabled ? 'bg-gray-100' : 'bg-gray-50 group-hover:bg-black group-hover:text-white'
      }`}>
        {icon}
      </div>
      <div className="flex-1">
        <div className="flex items-center gap-2 mb-1">
          <span className="font-black text-sm tracking-tight text-[#1C1C1E]">{title}</span>
          {recommended && (
            <span className="flex items-center gap-1 bg-black text-white text-[7px] font-black uppercase tracking-widest px-2 py-0.5 rounded-full">
              <Zap className="w-2 h-2 fill-current" /> Recommended
            </span>
          )}
        </div>
        <p className="text-[10px] text-gray-400 font-medium leading-relaxed max-w-[200px]">{description}</p>
      </div>
    </div>
  </button>
);

export default RunModelChoice;
