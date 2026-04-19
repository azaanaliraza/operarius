import React, { useState, useEffect } from 'react';
import { X, CheckCircle, Trash2, Info, Lock, ArrowUpRight, Activity } from 'lucide-react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import panther from '../assets/panther.png';

const apps = [
  { name: "Telegram", icon: "📨", needs: "Bot Token from @BotFather", how: "1. Open Telegram → search @BotFather\n2. /newbot → give name → copy token" },
  { name: "Slack", icon: "💬", needs: "Bot User OAuth Token (xoxb-...)", how: "1. Create Slack App → Socket Mode ON\n2. Install to workspace → copy Bot Token" },
  { name: "Gmail", icon: "✉️", needs: "OAuth Token (via Google)", how: "Hermes will open browser for OAuth. Just approve." },
  { name: "WhatsApp", icon: "📱", needs: "Business API Token", how: "Meta Business Manager → WhatsApp → API Setup → copy permanent token" },
  { name: "Signal", icon: "🔒", needs: "Signal CLI linked", how: "Link your Signal desktop via signal-cli" },
  { name: "Discord", icon: "🎮", needs: "Bot Token", how: "Discord Developer Portal → Bot → copy token" },
  { name: "Cal.com", icon: "📅", needs: "API Key", how: "Cal.com Settings → API Keys → create new" },
];

const ConnectPanel: React.FC<{ onClose: () => void }> = ({ onClose }) => {
  const [connected, setConnected] = useState<string[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [tokenInput, setTokenInput] = useState('');
  const [isSaving, setIsSaving] = useState(false);
  const [statusLog, setStatusLog] = useState<string | null>(null);

  useEffect(() => {
    invoke<string[]>('get_connected_apps').then(setConnected).catch(console.error);

    const unlistenStatus = listen<string>('hermes-status', (event) => {
      setStatusLog(event.payload);
      setTimeout(() => setStatusLog(null), 5000);
    });

    return () => { unlistenStatus.then(f => f()); };
  }, []);

  const saveToken = async () => {
    if (!selected || !tokenInput) return;
    setIsSaving(true);
    try {
      await invoke('save_app_token', { service: selected, token: tokenInput });
      setConnected(prev => [...prev, selected]);
      setTokenInput('');
      setSelected(null);
    } catch (e) {
      console.error(e);
      setStatusLog(`ERROR: ${String(e).toUpperCase()}`);
    } finally {
      setIsSaving(false);
    }
  };

  const removeToken = async (service: string) => {
    try {
      await invoke('remove_app_token', { service });
      setConnected(prev => prev.filter(s => s !== service));
    } catch (e) { console.error(e); }
  };

  return (
    <div className="fixed inset-0 bg-black/80 backdrop-blur-md flex items-center justify-center z-50 p-6 selection:bg-black selection:text-white">
      {/* STATUS TOAST PORTAL */}
      {statusLog && (
        <div className="fixed top-12 left-1/2 -translate-x-1/2 z-[60] bg-black border border-white/20 px-8 py-3 rounded-2xl shadow-2xl flex items-center gap-4 animate-in slide-in-from-top-4 duration-500">
           <Activity className="w-4 h-4 text-emerald-500 animate-pulse" />
           <span className="font-mono text-[9px] font-black uppercase tracking-[0.4em] text-white italic">{statusLog}</span>
        </div>
      )}

      <div className="bg-[#F8F9FA] w-full max-w-[540px] rounded-[2.5rem] p-8 max-h-[90vh] overflow-y-auto no-scrollbar border border-white/20 shadow-2xl relative animate-in fade-in zoom-in-95 duration-500">
        
        <div className="flex justify-between items-start mb-8">
          <div className="flex items-center gap-3">
            <img src={panther} className="w-8 h-8 object-contain brightness-0 grayscale opacity-80" />
            <div>
              <h2 className="text-xl font-black text-[#1C1C1E] tracking-tighter uppercase italic">Link Skills</h2>
              <div className="flex items-center gap-2 mt-0.5">
                 <Lock className="w-2.5 h-2.5 text-emerald-500" />
                 <span className="text-[8px] font-black text-gray-400 uppercase tracking-widest">Local-First Vault: Active</span>
              </div>
            </div>
          </div>
          <button onClick={onClose} className="w-8 h-8 bg-white rounded-full flex items-center justify-center border border-gray-100 shadow-sm hover:scale-110 active:scale-95 transition-all">
            <X className="w-4 h-4 text-black" />
          </button>
        </div>

        <div className="grid grid-cols-1 gap-2 mb-4">
          {apps.map(app => {
            const isConnected = connected.includes(app.name);
            const isPicking = selected === app.name;
            return (
              <div key={app.name} className={`group transition-all duration-500 rounded-[1.8rem] border overflow-hidden
                ${isConnected ? 'bg-emerald-50 border-emerald-100 shadow-inner' : isPicking ? 'bg-white border-black shadow-2xl' : 'bg-white border-gray-50 hover:border-black/10 hover:shadow-lg'}`}>
                <div className="p-4 flex items-center justify-between">
                  <div className="flex items-center gap-4 min-w-0">
                    <div className="text-2xl grayscale group-hover:grayscale-0 transition-all">{app.icon}</div>
                    <div className="min-w-0">
                      <div className="font-black text-xs text-[#1C1C1E] uppercase tracking-tight">{app.name}</div>
                      {isConnected ? (
                        <div className="text-emerald-600 text-[8px] font-black uppercase tracking-widest flex items-center gap-1.5 mt-0.5 animate-in fade-in">
                          <CheckCircle className="w-2.5 h-2.5" /> SECURE_LINK_ACTIVE
                        </div>
                      ) : (
                        <div className="text-[8px] text-gray-400 font-bold uppercase tracking-wider truncate mt-0.5 opacity-60 italic">{app.needs}</div>
                      )}
                    </div>
                  </div>
                  
                  {isConnected ? (
                    <button onClick={() => removeToken(app.name)} className="w-8 h-8 bg-rose-50 rounded-full flex items-center justify-center opacity-0 group-hover:opacity-100 transition-all hover:bg-rose-100 active:scale-90 shadow-sm">
                      <Trash2 className="w-3 h-3 text-rose-500" />
                    </button>
                  ) : (
                    <button onClick={() => setSelected(isPicking ? null : app.name)} className={`w-8 h-8 rounded-full flex items-center justify-center transition-all shadow-sm ${isPicking ? 'bg-black text-white' : 'bg-white border border-gray-100 group-hover:border-black'}`}>
                      <ArrowUpRight className="w-3.5 h-3.5" />
                    </button>
                  )}
                </div>

                {isPicking && !isConnected && (
                  <div className="px-5 pb-6 pt-2 animate-in slide-in-from-top-4 duration-300">
                    <div className="bg-gray-50/80 rounded-2xl p-4 border border-gray-100/50 mb-4">
                       <div className="flex items-center gap-2 text-[8px] font-black text-gray-500 mb-2 uppercase tracking-widest">
                         <Info className="w-2.5 h-2.5" /> Provisioning Instructions
                       </div>
                       <pre className="font-mono text-[9px] text-[#4B5563] leading-relaxed whitespace-pre-wrap">{app.how}</pre>
                    </div>
                    <div className="relative group/input">
                      <input
                        autoFocus
                        type="password"
                        value={tokenInput}
                        onChange={e => setTokenInput(e.target.value)}
                        className="w-full bg-gray-50/50 px-5 py-4 border border-gray-100 rounded-2xl focus:border-black focus:bg-white text-[10px] font-mono tracking-widest outline-none transition-all placeholder:tracking-normal placeholder:opacity-30"
                        placeholder="PASTE_SECURE_TOKEN_HERE..."
                      />
                    </div>
                    <button 
                      onClick={saveToken} 
                      disabled={isSaving}
                      className="mt-3 w-full bg-black text-white py-4 rounded-2xl font-black text-[9px] uppercase tracking-[0.3em] hover:shadow-2xl active:scale-[0.98] transition-all disabled:opacity-50"
                    >
                      {isSaving ? "CONFIGURING..." : "ANCHOR_LINK_TO_HERMES"}
                    </button>
                  </div>
                )}
              </div>
            );
          })}
        </div>

        <div className="mt-8 flex items-center justify-center gap-4 opacity-10">
           <div className="h-[0.5px] w-8 bg-black"></div>
           <span className="text-[7px] font-black uppercase tracking-[0.6em] text-black italic">Operarius Skill-Sync v2.2</span>
           <div className="h-[0.5px] w-8 bg-black"></div>
        </div>
      </div>
    </div>
  );
};

export default ConnectPanel;
