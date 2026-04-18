import React, { useState } from 'react';
import { X, CheckCircle, AlertCircle } from 'lucide-react';

const apps = [
  { name: "Gmail", icon: "✉️", needs: "OAuth or App Pass" },
  { name: "Slack", icon: "💬", needs: "Bot OAuth Token" },
  { name: "WhatsApp", icon: "📱", needs: "Business API Key" },
  { name: "Cal.com", icon: "📅", needs: "API Key" },
  { name: "Discord", icon: "🎮", needs: "Bot Token" },
];

const ConnectPanel: React.FC<{ onClose: () => void }> = ({ onClose }) => {
  const [selected, setSelected] = useState<string | null>(null);
  const [token, setToken] = useState('');
  const [status, setStatus] = useState<'idle' | 'valid' | 'invalid'>('idle');

  const validateToken = async () => {
    setStatus('valid');
  };

  return (
    <div className="fixed inset-0 bg-black/40 backdrop-blur-[2px] flex items-center justify-center z-50 p-6 font-sans">
      <div className="bg-white w-full max-w-[380px] rounded-[2rem] p-8 md:p-10 shadow-2xl animate-in fade-in zoom-in duration-300 relative border border-gray-100/50">
        <div className="flex justify-between items-center mb-6">
          <div>
            <h2 className="text-xl font-bold tracking-tight text-[#1C1C1E]">Connect Hub</h2>
            <p className="text-[10px] text-[#9CA3AF] font-bold uppercase tracking-widest mt-0.5">Local Authentication</p>
          </div>
          <button 
            onClick={onClose}
            className="w-8 h-8 bg-gray-50 rounded-full flex items-center justify-center hover:bg-black hover:text-white transition-all shadow-sm"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <p className="text-[10px] text-[#6B7280] mb-6 font-bold uppercase tracking-wider leading-relaxed opacity-60">
          All tokens sit in your OS native keychain.
        </p>

        <div className="space-y-2 max-h-[30vh] overflow-y-auto no-scrollbar pr-1">
          {apps.map(app => (
            <div
              key={app.name}
              onClick={() => setSelected(app.name)}
              className={`p-4 border border-gray-100 rounded-xl flex items-center gap-4 cursor-pointer transition-all duration-300 group
                ${selected === app.name ? 'border-black bg-gray-50' : 'hover:border-gray-300 bg-white shadow-sm hover:shadow-md'}`}
            >
              <div className="text-2xl group-hover:scale-110 transition-transform">{app.icon}</div>
              <div className="flex-1">
                <div className="font-bold text-sm text-[#1C1C1E]">{app.name}</div>
                <div className="text-[8px] text-[#9CA3AF] font-bold uppercase tracking-widest">{app.needs}</div>
              </div>
              {selected === app.name && <CheckCircle className="w-4 h-4 text-emerald-500" />}
            </div>
          ))}
        </div>

        {selected && (
          <div className="mt-8 pt-6 border-t border-gray-100 animate-in slide-in-from-bottom-2 duration-400">
            <label className="block text-[8px] font-bold text-[#9CA3AF] uppercase tracking-[0.2em] mb-2">PASTE {selected} KEY</label>
            <input
              type="password"
              value={token}
              onChange={(e) => setToken(e.target.value)}
              className="w-full px-4 py-2.5 bg-gray-50 border border-transparent focus:border-black rounded-lg focus:outline-none focus:bg-white transition-all font-mono text-xs mb-4"
              placeholder="••••••••••••"
            />
            
            <button 
              onClick={validateToken}
              className="w-full bg-black text-white py-3 rounded-xl font-bold text-xs shadow-lg hover:bg-[#2D2D2E] active:scale-[0.98] transition-all"
            >
              Verify Connection
            </button>

            {status === 'valid' && (
              <div className="mt-4 p-3 bg-emerald-50 border border-emerald-100 rounded-lg flex items-center gap-2 animate-in fade-in">
                <CheckCircle className="w-3.5 h-3.5 text-emerald-600" />
                <span className="text-[9px] font-bold text-emerald-700 uppercase tracking-widest">Securely Bonded</span>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};

export default ConnectPanel;
