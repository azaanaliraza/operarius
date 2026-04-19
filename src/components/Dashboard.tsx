import React, { useState, useEffect } from 'react';
import { 
  Cpu, HardDrive, Zap, Settings, Shield, 
  Activity, Plus, 
  ChevronRight, LayoutGrid, MessageSquare,
  Radio, Sparkles
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import ConnectPanel from './ConnectPanel';
import RagChat from './RagChat';

const Dashboard: React.FC = () => {
  const [hw, setHw] = useState<any>(null);
  const [showConnect, setShowConnect] = useState(false);
  const [engineStatus, setEngineStatus] = useState<'BOOTING' | 'ACTIVE' | 'ERROR'>('BOOTING');
  const [activeTab, setActiveTab] = useState<'fleet' | 'neural'>('fleet');

  const bootedRef = React.useRef(false);

  useEffect(() => {
    if (bootedRef.current) return;
    bootedRef.current = true;

    // 1. Scan hardware
    invoke('scan_hardware').then(setHw).catch(console.error);

    // 2. Start inference server + hermes gateway
    const boot = async () => {
      try {
        console.log("[Dashboard] Starting inference server...");
        await invoke('start_inference_server', { modelPath: '' });
        setEngineStatus('ACTIVE');
        console.log("[Dashboard] Inference server ready");

      } catch (e) {
        console.error("[Dashboard] Engine boot failed:", e);
        setEngineStatus('ERROR');
      }
    };
    boot();
  }, []);

  return (
    <div className="flex h-screen bg-[#F8F9FA] text-[#1C1C1E] font-sans selection:bg-black selection:text-white overflow-hidden">
      {/* SIDEBAR */}
      <aside className="w-[60px] border-r border-[#E5E5E7] flex flex-col items-center py-6 gap-8 bg-white/50 backdrop-blur-xl shrink-0">
        <div className="w-8 h-8 bg-black rounded-xl flex items-center justify-center shadow-lg hover:scale-110 transition-all cursor-pointer">
           <Zap className="w-4 h-4 text-white fill-white" />
        </div>
        
        <nav className="flex flex-col gap-6">
          <div 
            onClick={() => setActiveTab('fleet')}
            className={`p-2 rounded-lg transition-all cursor-pointer ${activeTab === 'fleet' ? 'bg-black text-white shadow-md' : 'text-gray-400 hover:bg-black/5'}`}
          > 
            <LayoutGrid className="w-5 h-5" /> 
          </div>
          <div 
            onClick={() => setActiveTab('neural')}
            className={`p-2 rounded-lg transition-all cursor-pointer ${activeTab === 'neural' ? 'bg-black text-white shadow-md' : 'text-gray-400 hover:bg-black/5'}`}
          > 
            <Sparkles className="w-5 h-5" /> 
          </div>
          <div className="p-2 text-gray-400 hover:bg-black/5 rounded-lg transition-all cursor-pointer" onClick={() => setShowConnect(true)}> <Radio className="w-5 h-5" /> </div>
          <div className="p-2 text-gray-400 hover:bg-black/5 rounded-lg transition-all cursor-pointer"> <Shield className="w-5 h-5" /> </div>
        </nav>

        <div className="mt-auto flex flex-col gap-6">
          <div className={`p-2 rounded-lg transition-all relative ${engineStatus === 'ACTIVE' ? 'text-emerald-500' : engineStatus === 'ERROR' ? 'text-red-500' : 'text-gray-300'}`}>
            <Activity className={`w-5 h-5 ${engineStatus === 'BOOTING' ? 'animate-pulse' : ''}`} />
            {engineStatus === 'ACTIVE' && <div className="absolute top-1 right-1 w-1.5 h-1.5 bg-emerald-500 rounded-full border border-white shadow-sm"></div>}
          </div>
          <div className="p-2 text-gray-400 hover:bg-black/5 rounded-lg transition-all cursor-pointer"> <Settings className="w-5 h-5" /> </div>
        </div>
      </aside>

      {/* MAIN */}
      <main className="flex-1 flex flex-col min-w-0 bg-white">
        <header className="h-[72px] px-8 flex items-center justify-between border-b border-[#E5E5E7] bg-white/30 backdrop-blur-md shrink-0">
          <div className="flex items-center gap-4">
             <h1 className="text-sm font-black tracking-widest uppercase italic bg-black text-white px-3 py-1 rounded">
               {activeTab === 'fleet' ? 'Fleet Management' : 'Neural Recall'}
             </h1>
             <div className="h-4 w-[1px] bg-gray-200"></div>
             <span className="text-[10px] font-bold text-gray-400 uppercase tracking-widest">{hw?.cpu_brand || "Scanning silicon..."}</span>
          </div>
          <div className="flex items-center gap-3">
             <div className={`text-[9px] font-black uppercase tracking-widest px-3 py-1.5 rounded-full border ${
               engineStatus === 'ACTIVE' ? 'bg-emerald-50 border-emerald-200 text-emerald-600' :
               engineStatus === 'ERROR' ? 'bg-red-50 border-red-200 text-red-600' :
               'bg-gray-50 border-gray-200 text-gray-400 animate-pulse'
             }`}>
               {engineStatus === 'ACTIVE' ? '● Engine Online' : engineStatus === 'ERROR' ? '● Engine Error' : '◌ Booting...'}
             </div>
          </div>
        </header>

        <div className="flex-1 overflow-hidden relative">
          {!hw ? (
            <div className="h-full w-full flex items-center justify-center">
              <div className="animate-pulse text-[10px] font-black uppercase tracking-widest text-gray-300">Synchronizing Silicon...</div>
            </div>
          ) : activeTab === 'fleet' ? (
            <div className="h-full overflow-y-auto p-8 no-scrollbar bg-gradient-to-b from-white/30 to-transparent">
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 mb-8">
                <StatCard icon={<Cpu className="w-4 h-4" />} label="Logic" val={hw.cpu_cores + " Cores"} sub="Optimized for Silicon" />
                <StatCard icon={<Zap className="w-4 h-4" />} label="Neural Memory" val={hw.ram_gb + " GB"} sub="Unified LPDDR5" />
                <StatCard icon={<HardDrive className="w-4 h-4" />} label="Vault" val={hw.storage_free_gb + " GB Free"} sub="Local RAG-Ready" />
              </div>

              <div className="grid grid-cols-1 lg:grid-cols-3 gap-8 text-left">
                <div className="lg:col-span-2 space-y-6">
                  <div className="flex items-center justify-between">
                    <h3 className="text-xs font-black uppercase tracking-[0.3em] text-gray-400">Deployed Agents</h3>
                    <button 
                      onClick={() => setActiveTab('neural')} 
                      className="flex items-center gap-2 text-[10px] font-black uppercase hover:text-black transition-all"
                    >
                      <MessageSquare className="w-3 h-3" /> Open Chat
                    </button>
                  </div>
                  
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <AgentCard name="Operarius Core" type="Local LLM" status={engineStatus === 'ACTIVE' ? 'Ready' : 'Booting'} desc="Direct inference via Metal GPU acceleration." />
                    <AgentCard name="Knowledge Agent" type="RAG Specialist" status="Ready" desc="Reads from ~/Documents/Operarius/knowledge/" />
                  </div>
                </div>

                <div className="space-y-6">
                   <h3 className="text-xs font-black uppercase tracking-[0.3em] text-gray-400">System Telemetry</h3>
                   <div className="bg-white border border-[#E5E5E7] rounded-3xl p-6 min-h-[300px] shadow-sm flex flex-col gap-4">
                      <TelemetryItem label="Inference Server" status={engineStatus === 'ACTIVE' ? "Online" : "Booting"} color={engineStatus === 'ACTIVE' ? "emerald" : "amber"} />
                      <TelemetryItem label="Knowledge Index" status="Standby" color="amber" />
                      <TelemetryItem label="Hermes Gateway" status={engineStatus === 'ACTIVE' ? "Connected" : "Waiting"} color={engineStatus === 'ACTIVE' ? "emerald" : "blue"} />
                      <div className="mt-auto pt-4 border-t border-dashed border-gray-100">
                                          <button 
                                            onClick={() => setShowConnect(true)} 
                                            className="text-[10px] font-black uppercase tracking-widest text-gray-400 hover:text-black transition-all flex items-center gap-2"
                                          >
                                            <Plus className="w-3 h-3" /> Connect Telegram
                                          </button>
                      </div>
                   </div>
                </div>
              </div>
            </div>
          ) : (
            <RagChat />
          )}
        </div>
      </main>

      {showConnect && <ConnectPanel onClose={() => setShowConnect(false)} />}
    </div>
  );
};

const StatCard = ({ icon, label, val, sub }: any) => (
  <div className="bg-white border border-[#E5E5E7] p-6 rounded-[2rem] shadow-sm hover:shadow-xl hover:-translate-y-1 transition-all group">
    <div className="flex items-start justify-between mb-4">
      <div className="w-10 h-10 bg-gray-50 rounded-2xl flex items-center justify-center group-hover:bg-black group-hover:text-white transition-all">
        {icon}
      </div>
      <div className="p-1 px-2 border border-gray-100 rounded-lg text-[8px] font-black uppercase text-gray-400 tracking-tighter">Live</div>
    </div>
    <div className="text-[10px] font-black text-gray-400 uppercase tracking-widest mb-1">{label}</div>
    <div className="text-2xl font-black text-[#1C1C1E] tracking-tighter mb-1">{val}</div>
    <div className="text-[10px] text-gray-400 font-medium">{sub}</div>
  </div>
);

const AgentCard = ({ name, type: agentType, status, desc }: any) => (
  <div className="bg-white border border-[#E5E5E7] p-6 rounded-[2.5rem] shadow-sm hover:border-black/20 transition-all cursor-pointer group">
    <div className="flex justify-between items-start mb-4">
      <div className="flex items-center gap-2">
        <div className={`w-2 h-2 rounded-full ${status === 'Ready' ? 'bg-emerald-500' : 'bg-amber-500 animate-pulse'}`}></div>
        <span className="text-[9px] font-black uppercase tracking-widest text-gray-400">{status}</span>
      </div>
      <ChevronRight className="w-4 h-4 text-gray-300 group-hover:translate-x-1 transition-all" />
    </div>
    <h4 className="text-sm font-black text-black mb-1 italic uppercase tracking-tighter">{name}</h4>
    <div className="text-[9px] font-bold text-gray-400 uppercase tracking-widest mb-3">{agentType}</div>
    <p className="text-[11px] text-gray-500 leading-relaxed font-medium">{desc}</p>
  </div>
);

const TelemetryItem = ({ label, status, color }: any) => {
  const colorMap: Record<string, string> = {
    emerald: 'bg-emerald-50 text-emerald-600',
    amber: 'bg-amber-50 text-amber-600',
    blue: 'bg-blue-50 text-blue-600',
    red: 'bg-red-50 text-red-600',
  };
  const dotMap: Record<string, string> = {
    emerald: 'bg-emerald-500',
    amber: 'bg-amber-500',
    blue: 'bg-blue-500',
    red: 'bg-red-500',
  };
  return (
    <div className="flex items-center justify-between group cursor-pointer hover:bg-gray-50 p-2 -m-2 rounded-xl transition-all">
      <span className="text-[11px] font-bold text-gray-500">{label}</span>
      <div className={`flex items-center gap-2 px-2 py-1 rounded-full text-[8px] font-black uppercase tracking-widest ${colorMap[color] || 'bg-gray-50 text-gray-600'}`}>
        <div className={`w-1 h-1 rounded-full ${dotMap[color] || 'bg-gray-500'}`}></div>
        {status}
      </div>
    </div>
  );
};

export default Dashboard;
