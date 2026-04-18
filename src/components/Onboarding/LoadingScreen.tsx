import React from 'react';
import panther from '../../assets/panther.png';

const LoadingScreen: React.FC<{ onRetry?: () => void }> = ({ onRetry }) => {
  return (
    <div className="h-screen w-screen bg-[#F8F9FA] flex flex-col items-center justify-center p-8 text-center font-sans overflow-hidden animate-in fade-in duration-700">
      
      {/* Floating Precision Mark */}
      <div className="absolute top-12 left-12 flex items-center gap-2 opacity-20">
        <div className="w-1.5 h-1.5 bg-black rounded-full animate-pulse"></div>
        <span className="text-[8px] font-black uppercase tracking-[0.3em]">Operarius local node</span>
      </div>

      <div className="text-[#1C1C1E] text-4xl md:text-5xl font-black tracking-tighter mb-10 animate-in slide-in-from-bottom-6 duration-1000 uppercase italic">
        Operarius
      </div>
      
      <div className="relative mb-12 group transition-all duration-700">
        <div className="absolute inset-0 bg-black/5 rounded-full blur-3xl -z-10 group-hover:scale-110 transition-transform"></div>
        <img 
          src={panther} 
          alt="Operarius Panther" 
          className="w-32 h-32 md:w-36 md:h-36 object-contain drop-shadow-[0_20px_50px_rgba(0,0,0,0.15)] animate-in zoom-in-75 duration-1000" 
        />
      </div>
      
      <div className="w-48 h-1 bg-gray-100 rounded-full overflow-hidden relative shadow-inner">
        <div className="h-full w-1/2 bg-black animate-[progress_1.5s_ease-in-out_infinite] absolute top-0 left-0 rounded-full"></div>
      </div>
      
      <div className="flex flex-col items-center mt-10 space-y-4">
        <p className="text-[#9CA3AF] text-[10px] font-black uppercase tracking-[0.4em] opacity-40 animate-pulse">
          Synchronizing Core Infrastructure
        </p>
        
        {onRetry && (
          <button 
            onClick={onRetry}
            className="text-[8px] font-black uppercase tracking-widest text-[#1C1C1E]/30 hover:text-black transition-colors"
          >
            Click to force continue →
          </button>
        )}
      </div>

      {/* Dynamic Background Noise */}
      <div className="absolute inset-0 pointer-events-none opacity-[0.03] grayscale contrast-200" style={{ backgroundImage: `url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E")` }}></div>
    </div>
  );
};

export default LoadingScreen;
