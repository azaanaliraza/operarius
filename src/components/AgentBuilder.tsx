import React, { useState } from 'react';
import { X, ArrowRight, Zap, Target, BookOpen, ChevronRight, Play, Save } from 'lucide-react';

interface Node {
  id: string;
  type: 'trigger' | 'action' | 'condition';
  label: string;
  desc: string;
}

const AgentBuilder: React.FC<{ onClose: () => void }> = ({ onClose }) => {
  const [prompt, setPrompt] = useState("");
  const [step, setStep] = useState(1); // 1: input, 2: logic, 3: teach
  
  const nodes: Node[] = [
    { id: '1', type: 'trigger', label: 'Monitor Gmail', desc: 'Watch for "Urgent" invoices' },
    { id: '2', type: 'condition', label: 'Check Total', desc: 'Is amount > $500?' },
    { id: '3', type: 'action', label: 'Draft Reply', desc: 'Ask for payment terms' },
    { id: '4', type: 'action', label: 'Notify Slack', desc: 'Ping Finance channel' },
  ];

  return (
    <div className="fixed inset-0 bg-white/60 backdrop-blur-xl z-[100] flex items-center justify-center p-6 md:p-12 animate-in fade-in duration-500 selection:bg-black selection:text-white">
      <div className="w-full max-w-5xl bg-white rounded-[3rem] shadow-2xl border border-gray-100 flex flex-col h-full max-h-[85vh] overflow-hidden relative">
        
        {/* Header */}
        <div className="p-8 border-b border-gray-50 flex items-center justify-between">
          <div>
            <div className="text-[10px] font-bold text-gray-400 uppercase tracking-widest mb-1">Phase 2</div>
            <h2 className="text-xl font-black tracking-tighter text-[#1C1C1E]">AGENT BUILDER</h2>
          </div>
          <button onClick={onClose} className="w-10 h-10 bg-gray-50 rounded-full flex items-center justify-center hover:bg-black hover:text-white transition-all group">
            <X className="w-5 h-5 group-hover:rotate-90 transition-transform" />
          </button>
        </div>

        {/* Content Area */}
        <div className="flex-1 flex overflow-hidden">
          
          {/* Left: Teaching Side (Logic & Input) */}
          <div className="w-2/5 border-r border-gray-50 p-10 flex flex-col gap-8 overflow-y-auto no-scrollbar bg-[#FBFBFC]">
            
            <section>
              <h3 className="text-[10px] font-bold text-black uppercase tracking-widest mb-4">Command Input</h3>
              <textarea 
                value={prompt}
                onChange={(e) => setPrompt(e.target.value)}
                placeholder="Ex: When I get an urgent invoice from Google, draft a reply and notify the finance channel on Slack."
                className="w-full h-32 bg-white border border-gray-100 rounded-2xl p-5 text-sm font-medium focus:ring-2 focus:ring-black outline-none transition-all resize-none shadow-sm"
              />
            </section>

            <section className="flex-1">
              <h3 className="text-[10px] font-bold text-black uppercase tracking-widest mb-4">Logic Breakdown</h3>
              <div className="space-y-3">
                {nodes.map((node, i) => (
                  <div key={node.id} className="flex items-start gap-4 animate-in slide-in-from-left-4 duration-500" style={{ animationDelay: `${i * 100}ms` }}>
                    <div className={`w-6 h-6 rounded-full flex items-center justify-center text-[10px] font-black text-white shrink-0 ${
                      node.type === 'trigger' ? 'bg-black' : node.type === 'condition' ? 'bg-amber-500' : 'bg-emerald-500'
                    }`}>
                      {i + 1}
                    </div>
                    <div>
                      <div className="text-[11px] font-bold text-[#1C1C1E] uppercase tracking-tighter">{node.label}</div>
                      <p className="text-[10px] text-gray-400 font-medium leading-tight">{node.desc}</p>
                    </div>
                  </div>
                ))}
              </div>
            </section>

            <div className="flex gap-2">
              <button className="flex-1 bg-black text-white py-4 rounded-2xl font-bold flex items-center justify-center gap-2 hover:bg-[#2D2D2E] transition-all text-xs active:scale-95 shadow-lg">
                <Play className="w-4 h-4 fill-current" /> Deploy Agent
              </button>
              <button className="w-16 bg-white border border-gray-100 rounded-2xl flex items-center justify-center hover:border-black transition-all group shadow-sm">
                <Save className="w-5 h-5 text-gray-400 group-hover:text-black" />
              </button>
            </div>
          </div>

          {/* Right: Visual Logic Canvas */}
          <div className="flex-1 bg-white relative overflow-hidden flex items-center justify-center p-12">
            
            <div className="absolute inset-0 opacity-[0.03] pointer-events-none" style={{ backgroundImage: 'radial-gradient(#000 1px, transparent 0)', backgroundSize: '24px 24px' }}></div>

            <div className="relative z-10 flex flex-col items-center gap-12">
              {nodes.map((node, i) => (
                <React.Fragment key={node.id}>
                  <div className="w-64 bg-white border border-gray-100 p-6 rounded-[2rem] shadow-xl animate-in zoom-in-95 duration-700 hover:border-black transition-all cursor-default group">
                     <div className="flex items-center gap-4 mb-3">
                        <div className={`w-8 h-8 rounded-xl flex items-center justify-center ${
                           node.type === 'trigger' ? 'bg-black text-white' : 
                           node.type === 'condition' ? 'bg-amber-50 text-amber-600' : 
                           'bg-emerald-50 text-emerald-600'
                        }`}>
                           {node.type === 'trigger' ? <Zap className="w-4 h-4" /> : node.type === 'condition' ? <Target className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
                        </div>
                        <div>
                          <div className="text-[8px] font-bold text-gray-300 uppercase tracking-widest leading-none mb-1">{node.type}</div>
                          <div className="font-bold text-xs tracking-tight uppercase">{node.label}</div>
                        </div>
                     </div>
                     <p className="text-[10px] text-gray-400 font-medium leading-relaxed">{node.desc}</p>
                  </div>
                  {i < nodes.length - 1 && (
                    <div className="w-px h-12 bg-gradient-to-b from-gray-100 to-transparent relative">
                      <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-2 h-2 bg-gray-100 rounded-full"></div>
                    </div>
                  )}
                </React.Fragment>
              ))}
            </div>

            <div className="absolute bottom-8 right-8 flex gap-2">
               <div className="px-3 py-1 bg-gray-50 border border-gray-100 rounded-full text-[8px] font-bold uppercase tracking-widest text-gray-400">Zoom: 1.0x</div>
               <div className="px-3 py-1 bg-gray-50 border border-gray-100 rounded-full text-[8px] font-bold uppercase tracking-widest text-gray-400">Node count: {nodes.length}</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default AgentBuilder;
