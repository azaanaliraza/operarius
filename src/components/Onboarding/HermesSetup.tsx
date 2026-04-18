import React, { useState, useEffect } from 'react';
import { Check, Circle } from 'lucide-react';

const steps = [
  { id: 1, title: "Extracting runtime", desc: "Python backend + Hermes framework" },
  { id: 2, title: "Linking skills", desc: "Gmail • Slack • Cal.com • WhatsApp" },
  { id: 3, title: "Configuring sandbox", desc: "6-layer security isolation" },
  { id: 4, title: "Registering scheduler", desc: "launchd / Task Scheduler" },
];

const HermesSetup: React.FC<{ onComplete: () => void }> = ({ onComplete }) => {
  const [currentStep, setCurrentStep] = useState(0);

  useEffect(() => {
    const timer = setInterval(() => {
      setCurrentStep(s => {
        if (s >= steps.length - 1) {
          clearInterval(timer);
          setTimeout(onComplete, 1200);
          return s;
        }
        return s + 1;
      });
    }, 900);
    return () => clearInterval(timer);
  }, [onComplete]);

  return (
    <div className="h-screen bg-[#F8F9FA] flex flex-col items-center justify-center p-6 font-sans overflow-hidden">
      <div className="text-center mb-16 animate-in fade-in slide-in-from-top-4 duration-700">
        <div className="text-6xl mb-6 drop-shadow-lg transform hover:scale-110 transition-transform cursor-pointer">🐆</div>
        <h1 className="text-3xl md:text-4xl font-bold tracking-tight text-[#1C1C1E]">Setting up Hermes</h1>
        <p className="text-[#6B7280] mt-2 font-medium text-sm md:text-base">This only happens once.</p>
      </div>

      <div className="w-full max-w-sm space-y-4">
        {steps.map((step, i) => (
          <div key={step.id} className={`flex gap-5 items-start p-5 rounded-2xl transition-all duration-300 border ${
            i === currentStep ? 'bg-white border-black shadow-xl scale-105' : 
            i < currentStep ? 'bg-white/50 border-gray-100 opacity-60' : 'bg-white/30 border-transparent opacity-40'
          }`}>
            <div className={`w-7 h-7 rounded-full flex items-center justify-center flex-shrink-0 mt-0.5 transition-all
              ${i < currentStep ? 'bg-emerald-500 text-white' : i === currentStep ? 'bg-black text-white' : 'bg-gray-200 text-gray-400'}`}>
              {i < currentStep ? <Check className="w-4 h-4" /> : <Circle className="w-4 h-4" />}
            </div>
            <div>
              <div className={`font-bold text-sm transition-colors ${i === currentStep ? 'text-black' : 'text-gray-900'}`}>{step.title}</div>
              <div className="text-[11px] text-[#6B7280] font-medium leading-tight">{step.desc}</div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default HermesSetup;
