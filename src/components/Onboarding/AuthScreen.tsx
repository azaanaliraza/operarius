import React from 'react';

const AuthScreen: React.FC<{ onComplete: () => void }> = ({ onComplete }) => {
  return (
    <div className="h-screen w-screen bg-[#F8F9FA] flex flex-col items-center justify-center p-12 overflow-hidden selection:bg-black selection:text-white">
      <div className="w-full max-w-[360px] bg-white rounded-[2.5rem] p-10 shadow-2xl border border-gray-100/50 animate-in fade-in zoom-in-95 duration-700 text-center">
        
        <div className="mb-10">
          <div className="text-[10px] font-bold text-gray-400 uppercase tracking-[0.2em] mb-3 opacity-50">Local Intelligence</div>
          <h1 className="text-2xl font-black tracking-tighter text-[#1C1C1E]">OPERARIUS</h1>
        </div>

        <div className="text-5xl mb-12 transform hover:scale-110 transition-transform duration-700 cursor-default">🐆</div>

        <button 
          onClick={onComplete}
          className="w-full bg-[#1C1C1E] text-white py-4 rounded-2xl font-bold text-xs uppercase tracking-[0.2em] hover:bg-black transition-all shadow-xl active:scale-95"
        >
          Initialize Engine
        </button>

        <p className="text-[9px] text-gray-400 mt-10 px-6 leading-relaxed font-bold uppercase tracking-widest opacity-40">
          Secure local environment.<br />No cloud required.
        </p>
      </div>
    </div>
  );
};

export default AuthScreen;
