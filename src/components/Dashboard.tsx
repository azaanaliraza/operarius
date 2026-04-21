import React, { useState, useEffect } from 'react';
import {
  Cpu, HardDrive, Zap, Settings, Shield,
  Activity, Plus,
  ChevronRight, LayoutGrid, MessageSquare,
  Radio, Sparkles, Download, Trash2, RefreshCcw, Brain, MemoryStick, Search,
  Clock3, Terminal
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import ConnectPanel from './ConnectPanel';
import RagChat from './RagChat';
import AgentBuilder from './AgentBuilder/AgentBuilder';
import panther from '../assets/panther.png';

interface ModelCapability {
  id: string;
  name: string;
  runtime: string;
  file: string;
  repo: string;
  size_gb: number;
  estimated_ram_gb: number;
  installed: boolean;
  active: boolean;
  supports_vision: boolean;
  supports_tts: boolean;
  coding_strength: string;
  best_for: string[];
  notes: string;
}

interface UserPreference {
  key: string;
  value: string;
}

interface MemoryEntry {
  id: string;
  user_id: string;
  scope: string;
  memory_key: string;
  value: string;
  weight: number;
  updated_at: string;
}

interface UserSkill {
  id: string;
  user_id: string;
  name: string;
  description: string;
  instruction: string;
  is_active: number;
  triggers: string;
  updated_at: string;
}

interface SkillMarketplaceItem {
  name: string;
  built_in: boolean;
  registry: string;
  description: string;
  installed: boolean;
}

interface AgentLogEntry {
  id: string;
  ts: string;
  level: 'info' | 'warning' | 'error';
  scope: string;
  message: string;
  details?: Record<string, unknown>;
}

const USER_ID = 'local-user';

const Dashboard: React.FC<{ onOpenSetup: () => void }> = ({ onOpenSetup }) => {
  const [hw, setHw] = useState<{ cpu_brand: string; cpu_cores: number; ram_gb: number; storage_free_gb: number } | null>(null);
  const [showConnect, setShowConnect] = useState(false);
  const [engineStatus, setEngineStatus] = useState<'BOOTING' | 'ACTIVE' | 'ERROR'>('BOOTING');
  const [activeTab, setActiveTab] = useState<'fleet' | 'neural' | 'skills' | 'builder'>('fleet');

  const [catalog, setCatalog] = useState<ModelCapability[]>([]);
  const [customRepo, setCustomRepo] = useState('');
  const [customFile, setCustomFile] = useState('');
  const [busyModelFile, setBusyModelFile] = useState('');

  const [preferences, setPreferences] = useState<UserPreference[]>([]);
  const [prefKey, setPrefKey] = useState('');
  const [prefValue, setPrefValue] = useState('');

  const [memories, setMemories] = useState<MemoryEntry[]>([]);
  const [memoryScope, setMemoryScope] = useState('workstyle');
  const [memoryKey, setMemoryKey] = useState('');
  const [memoryValue, setMemoryValue] = useState('');

  const [skills, setSkills] = useState<UserSkill[]>([]);
  const [skillName, setSkillName] = useState('');
  const [skillDesc, setSkillDesc] = useState('');
  const [skillInstruction, setSkillInstruction] = useState('');

  const [marketplaceSkills, setMarketplaceSkills] = useState<SkillMarketplaceItem[]>([]);
  const [skillSearch, setSkillSearch] = useState('');
  const [marketBusy, setMarketBusy] = useState('');
  const [clock, setClock] = useState(new Date());
  const [agentLogs, setAgentLogs] = useState<AgentLogEntry[]>([]);
  const [lastResponseMs, setLastResponseMs] = useState<number | null>(null);
  const [agentHealth, setAgentHealth] = useState<'healthy' | 'busy' | 'degraded'>('busy');
  const [showDiagnostics, setShowDiagnostics] = useState(false);

  const bootedRef = React.useRef(false);

  const refreshModelCatalog = async () => {
    try {
      const items = await invoke<ModelCapability[]>('get_model_catalog');
      setCatalog(items);
    } catch (e) {
      console.error('Failed to fetch model catalog', e);
    }
  };

  const refreshMemorySkillState = async () => {
    try {
      const [prefs, mems, skl] = await Promise.all([
        invoke<UserPreference[]>('get_user_preferences', { userId: USER_ID }),
        invoke<MemoryEntry[]>('list_memory_entries', { userId: USER_ID }),
        invoke<UserSkill[]>('list_user_skills', { userId: USER_ID }),
      ]);
      setPreferences(prefs);
      setMemories(mems);
      setSkills(skl);
    } catch (e) {
      console.error('Failed to load memory/skills state', e);
    }
  };

  const refreshSkillMarketplace = async (query?: string) => {
    try {
      const items = await invoke<SkillMarketplaceItem[]>('fetch_skill_marketplace', {
        query: query || '',
      });
      setMarketplaceSkills(items);
    } catch (e) {
      console.error('Failed to load skill marketplace', e);
    }
  };

  const loadAgentLogs = async () => {
    try {
      const records = await invoke<AgentLogEntry[]>('get_agent_logs');
      setAgentLogs(records.map((record) => {
        const parsed = new Date(record.ts);
        return {
          ...record,
          ts: Number.isNaN(parsed.getTime())
            ? record.ts
            : parsed.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' }),
        };
      }));

      const latestLatency = [...records].find((record) => typeof record.details?.elapsed_ms === 'number');
      if (latestLatency && typeof latestLatency.details?.elapsed_ms === 'number') {
        setLastResponseMs(latestLatency.details.elapsed_ms as number);
        setAgentHealth((latestLatency.details.elapsed_ms as number) > 15000 ? 'busy' : 'healthy');
      }
    } catch (e) {
      console.error('Failed to load agent logs', e);
    }
  };

  useEffect(() => {
    if (bootedRef.current) return;
    bootedRef.current = true;

    invoke<{ cpu_brand: string; cpu_cores: number; ram_gb: number; storage_free_gb: number }>('scan_hardware').then(setHw).catch(console.error);

    const boot = async () => {
      try {
        await invoke('start_inference_server', { modelPath: '' });
        setEngineStatus('ACTIVE');
      } catch (e) {
        console.error('[Dashboard] Engine boot failed:', e);
        setEngineStatus('ERROR');
      }
    };

    boot();
    refreshModelCatalog();
    refreshMemorySkillState();
    refreshSkillMarketplace('');
    loadAgentLogs();
  }, []);

  useEffect(() => {
    const timer = setInterval(() => {
      setClock(new Date());
    }, 1000);

    return () => clearInterval(timer);
  }, []);

  useEffect(() => {
    let mounted = true;

    const addLog = (entry: Omit<AgentLogEntry, 'id' | 'ts'>) => {
      if (!mounted) return;

      const next: AgentLogEntry = {
        ...entry,
        id: `${Date.now()}-${Math.random().toString(16).slice(2)}`,
        ts: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' }),
      };

      setAgentLogs((current) => [next, ...current].slice(0, 200));

      if (entry.level === 'error') {
        setAgentHealth('degraded');
      } else if (entry.details && typeof entry.details.elapsed_ms === 'number') {
        const elapsedMs = entry.details.elapsed_ms as number;
        setLastResponseMs(elapsedMs);
        setAgentHealth(elapsedMs > 15000 ? 'busy' : 'healthy');
      }
    };

    const disposers: Array<Promise<() => void>> = [];
    disposers.push(listen<Record<string, unknown>>('agent-log', (event) => {
      const payload = event.payload || {};
      addLog({
        level: (payload.level as AgentLogEntry['level']) || 'info',
        scope: String(payload.scope || 'system'),
        message: String(payload.message || 'Agent event'),
        details: (payload.details as Record<string, unknown>) || {},
      });
    }));
    disposers.push(listen<Record<string, unknown>>('brain-status', (event) => {
      const payload = event.payload || {};
      const ready = Boolean(payload.ready);
      const context = payload.context;
      addLog({
        level: 'info',
        scope: 'brain',
        message: ready ? 'Brain is online' : 'Brain status update',
        details: { context },
      });
      if (ready) {
        setAgentHealth('healthy');
      }
    }));
    disposers.push(listen<Record<string, unknown>>('chat-history-updated', (event) => {
      const payload = event.payload || {};
      addLog({
        level: 'info',
        scope: String(payload.platform || 'app'),
        message: 'Conversation history synced',
      });
    }));

    return () => {
      mounted = false;
      disposers.forEach((disposePromise) => {
        disposePromise.then((dispose) => dispose()).catch(() => undefined);
      });
    };
  }, []);

  const switchModel = async (file: string) => {
    setBusyModelFile(file);
    try {
      setEngineStatus('BOOTING');
      await invoke('switch_active_model', { filename: file });
      setEngineStatus('ACTIVE');
      await refreshModelCatalog();
    } catch (e) {
      setEngineStatus('ERROR');
      console.error(e);
      alert(String(e));
    } finally {
      setBusyModelFile('');
    }
  };

  const downloadCatalogModel = async (repo: string, file: string) => {
    setBusyModelFile(file);
    try {
      await invoke('download_model', { repo, filename: file });
      await refreshModelCatalog();
    } catch (e) {
      console.error(e);
      alert(String(e));
    } finally {
      setBusyModelFile('');
    }
  };

  const deleteModel = async (file: string) => {
    setBusyModelFile(file);
    try {
      await invoke('delete_model_file', { filename: file });
      await refreshModelCatalog();
    } catch (e) {
      console.error(e);
      alert(String(e));
    } finally {
      setBusyModelFile('');
    }
  };

  const downloadCustomModel = async () => {
    if (!customRepo.trim() || !customFile.trim()) return;
    await downloadCatalogModel(customRepo.trim(), customFile.trim());
    setCustomRepo('');
    setCustomFile('');
  };

  const savePreference = async () => {
    if (!prefKey.trim() || !prefValue.trim()) return;
    await invoke('set_user_preference', { userId: USER_ID, key: prefKey.trim(), value: prefValue.trim() });
    setPrefKey('');
    setPrefValue('');
    await refreshMemorySkillState();
  };

  const saveMemory = async () => {
    if (!memoryKey.trim() || !memoryValue.trim()) return;
    await invoke('upsert_memory_entry', {
      userId: USER_ID,
      scope: memoryScope,
      memoryKey: memoryKey.trim(),
      value: memoryValue.trim(),
      weight: 75,
    });
    setMemoryKey('');
    setMemoryValue('');
    await refreshMemorySkillState();
  };

  const createSkill = async () => {
    if (!skillName.trim() || !skillInstruction.trim()) return;
    await invoke('create_user_skill', {
      userId: USER_ID,
      name: skillName.trim(),
      description: skillDesc.trim(),
      instruction: skillInstruction.trim(),
      triggers: '',
    });
    setSkillName('');
    setSkillDesc('');
    setSkillInstruction('');
    await refreshMemorySkillState();
  };

  const toggleSkill = async (id: string, current: number) => {
    await invoke('set_user_skill_active', { skillId: id, isActive: current !== 1 });
    await refreshMemorySkillState();
  };

  const removeSkill = async (id: string) => {
    await invoke('delete_user_skill', { skillId: id });
    await refreshMemorySkillState();
  };

  const installMarketplaceSkill = async (name: string) => {
    setMarketBusy(name);
    try {
      await invoke('install_marketplace_skill', { skillName: name });
      await refreshSkillMarketplace(skillSearch);
      await refreshMemorySkillState();
    } catch (e) {
      console.error(e);
      alert(String(e));
    } finally {
      setMarketBusy('');
    }
  };

  return (
    <div className="flex h-screen bg-[#F8F9FA] text-[#1C1C1E] font-sans selection:bg-black selection:text-white overflow-hidden">
      <aside className="w-[60px] border-r border-[#E5E5E7] flex flex-col items-center py-6 gap-8 bg-white/50 backdrop-blur-xl shrink-0">
        <div className="w-8 h-8 bg-black rounded-xl flex items-center justify-center shadow-lg hover:scale-110 transition-all cursor-pointer overflow-hidden">
          <img src={panther} alt="Operarius" className="w-5 h-5 object-contain brightness-0 invert" />
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
          <div
            onClick={() => setActiveTab('skills')}
            className={`p-2 rounded-lg transition-all cursor-pointer ${activeTab === 'skills' ? 'bg-black text-white shadow-md' : 'text-gray-400 hover:bg-black/5'}`}
          >
            <Brain className="w-5 h-5" />
          </div>
          <div
            onClick={() => setActiveTab('builder')}
            className={`p-2 rounded-lg transition-all cursor-pointer ${activeTab === 'builder' ? 'bg-black text-white shadow-md' : 'text-gray-400 hover:bg-black/5'}`}
          >
            <Terminal className="w-5 h-5" />
          </div>
          <div className="p-2 text-gray-400 hover:bg-black/5 rounded-lg transition-all cursor-pointer" onClick={() => setShowConnect(true)}>
            <Radio className="w-5 h-5" />
          </div>
          <div
            className={`p-2 rounded-lg transition-all cursor-pointer ${showDiagnostics ? 'bg-emerald-50 text-emerald-600 shadow-sm' : 'text-gray-400 hover:bg-black/5'}`}
            onClick={() => setShowDiagnostics((current) => !current)}
            title="Open agent diagnostics"
          >
            <Shield className="w-5 h-5" />
          </div>
        </nav>

        <div className="mt-auto flex flex-col gap-6">
          <div className={`p-2 rounded-lg transition-all relative ${engineStatus === 'ACTIVE' ? 'text-emerald-500' : engineStatus === 'ERROR' ? 'text-red-500' : 'text-gray-300'}`}>
            <Activity className={`w-5 h-5 ${engineStatus === 'BOOTING' ? 'animate-pulse' : ''}`} />
            {engineStatus === 'ACTIVE' && <div className="absolute top-1 right-1 w-1.5 h-1.5 bg-emerald-500 rounded-full border border-white shadow-sm"></div>}
          </div>
          <div
            className="p-2 text-gray-400 hover:bg-black/5 rounded-lg transition-all cursor-pointer"
            onClick={onOpenSetup}
            title="Re-open ignite engine setup"
          >
            <Settings className="w-5 h-5" />
          </div>
        </div>
      </aside>

      <main className="flex-1 flex flex-col min-w-0 bg-white">
        <header className="h-[72px] px-8 flex items-center justify-between border-b border-[#E5E5E7] bg-white/30 backdrop-blur-md shrink-0">
          <div className="flex items-center gap-4">
            <h1 className="text-sm font-black tracking-widest uppercase italic bg-black text-white px-3 py-1 rounded">
              {activeTab === 'fleet'
                ? 'Fleet Management'
                : activeTab === 'neural'
                  ? 'Neural Recall'
                  : activeTab === 'skills'
                    ? 'Skill Marketplace'
                    : 'Agent Builder'}
            </h1>
            <div className="h-4 w-[1px] bg-gray-200"></div>
            <span className="text-[10px] font-bold text-gray-400 uppercase tracking-widest">{hw?.cpu_brand || 'Scanning silicon...'}</span>
          </div>
          <div className="flex items-center gap-3">
            <button
              onClick={async () => {
                await refreshModelCatalog();
                await refreshMemorySkillState();
              }}
              className="text-[9px] font-black uppercase tracking-widest px-2 py-1 rounded-full border border-gray-200 text-gray-500 hover:text-black"
            >
              <RefreshCcw className="w-3 h-3 inline-block mr-1" /> Refresh
            </button>
            <div className="hidden md:flex items-center gap-2 text-[9px] font-black uppercase tracking-widest px-3 py-1.5 rounded-full border border-gray-200 text-gray-500 bg-white">
              <Clock3 className="w-3 h-3" />
              {clock.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })}
            </div>
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
            <div className="h-full overflow-y-auto p-8 no-scrollbar bg-gradient-to-b from-white/30 to-transparent space-y-8">
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                <StatCard icon={<Cpu className="w-4 h-4" />} label="Logic" val={hw.cpu_cores + ' Cores'} sub="Optimized for Silicon" />
                <StatCard icon={<Zap className="w-4 h-4" />} label="Neural Memory" val={hw.ram_gb + ' GB'} sub="Unified LPDDR5" />
                <StatCard icon={<HardDrive className="w-4 h-4" />} label="Vault" val={hw.storage_free_gb + ' GB Free'} sub="Local RAG-Ready" />
              </div>

              <section className="bg-white border border-[#E5E5E7] rounded-[2rem] p-6 shadow-sm">
                <div className="flex items-center justify-between mb-4">
                  <h3 className="text-xs font-black uppercase tracking-[0.3em] text-gray-500">Model Control Center</h3>
                  <button onClick={refreshModelCatalog} className="text-[10px] font-black uppercase tracking-widest text-gray-400 hover:text-black">
                    Refresh Catalog
                  </button>
                </div>

                <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
                  {catalog.map((model) => (
                    <div key={model.id} className="border border-gray-100 rounded-[1.4rem] p-4 hover:border-black/10 transition-all">
                      <div className="flex items-start justify-between gap-3">
                        <div>
                          <div className="text-sm font-black tracking-tight">{model.name}</div>
                          <div className="text-[10px] text-gray-500 font-bold uppercase tracking-widest mt-1">
                            {model.runtime} • {model.size_gb.toFixed(1)} GB • ~{model.estimated_ram_gb} GB RAM
                          </div>
                        </div>
                        {model.active ? (
                          <span className="text-[9px] font-black uppercase tracking-widest px-2 py-1 rounded-full bg-emerald-100 text-emerald-700">active</span>
                        ) : null}
                      </div>

                      <div className="mt-3 flex flex-wrap gap-2">
                        <CapChip label={model.supports_vision ? 'Vision' : 'No Vision'} enabled={model.supports_vision} />
                        <CapChip label={model.supports_tts ? 'TTS' : 'No TTS'} enabled={model.supports_tts} />
                        <CapChip label={`Coding: ${model.coding_strength}`} enabled={model.coding_strength !== 'basic'} />
                        {model.best_for.slice(0, 2).map((t) => (
                          <span key={t} className="text-[9px] font-black uppercase tracking-widest px-2 py-1 rounded-full bg-gray-100 text-gray-600">{t}</span>
                        ))}
                      </div>

                      <p className="text-[11px] text-gray-600 mt-3 leading-relaxed">{model.notes}</p>

                      <div className="mt-4 flex gap-2 flex-wrap">
                        {!model.installed ? (
                          <button
                            disabled={busyModelFile === model.file}
                            onClick={() => downloadCatalogModel(model.repo, model.file)}
                            className="text-[10px] font-black uppercase tracking-widest px-3 py-2 rounded-xl bg-black text-white disabled:opacity-50"
                          >
                            <Download className="w-3 h-3 inline-block mr-1" /> Download
                          </button>
                        ) : (
                          <button
                            disabled={busyModelFile === model.file || model.active || model.runtime !== 'llama.cpp'}
                            onClick={() => switchModel(model.file)}
                            className="text-[10px] font-black uppercase tracking-widest px-3 py-2 rounded-xl bg-emerald-600 text-white disabled:opacity-40"
                            title={model.runtime !== 'llama.cpp' ? 'Switching MLX model runtime is not wired to llama.cpp engine.' : ''}
                          >
                            <Zap className="w-3 h-3 inline-block mr-1" /> {model.active ? 'In Use' : 'Switch'}
                          </button>
                        )}
                        <button
                          disabled={busyModelFile === model.file || !model.installed}
                          onClick={() => deleteModel(model.file)}
                          className="text-[10px] font-black uppercase tracking-widest px-3 py-2 rounded-xl border border-rose-200 text-rose-600 disabled:opacity-40"
                        >
                          <Trash2 className="w-3 h-3 inline-block mr-1" /> Delete
                        </button>
                      </div>
                    </div>
                  ))}
                </div>

                <div className="mt-6 border-t border-dashed border-gray-200 pt-4">
                  <h4 className="text-[10px] font-black uppercase tracking-[0.2em] text-gray-500 mb-3">Add model by direct download</h4>
                  <div className="grid grid-cols-1 lg:grid-cols-3 gap-2">
                    <input
                      value={customRepo}
                      onChange={(e) => setCustomRepo(e.target.value)}
                      placeholder="HF repo, e.g. unsloth/MyModel"
                      className="border border-gray-200 rounded-xl px-3 py-2 text-xs outline-none"
                    />
                    <input
                      value={customFile}
                      onChange={(e) => setCustomFile(e.target.value)}
                      placeholder="filename.gguf"
                      className="border border-gray-200 rounded-xl px-3 py-2 text-xs outline-none"
                    />
                    <button
                      onClick={downloadCustomModel}
                      className="text-[10px] font-black uppercase tracking-widest px-3 py-2 rounded-xl bg-black text-white"
                    >
                      Download Custom
                    </button>
                  </div>
                </div>
              </section>

              <section className="grid grid-cols-1 lg:grid-cols-2 gap-6">
                <div className="bg-white border border-[#E5E5E7] rounded-[2rem] p-6 shadow-sm">
                  <div className="flex items-center gap-2 mb-4">
                    <MemoryStick className="w-4 h-4" />
                    <h3 className="text-xs font-black uppercase tracking-[0.3em] text-gray-500">Memory Manager</h3>
                  </div>

                  <div className="space-y-2 mb-4">
                    <div className="grid grid-cols-2 gap-2">
                      <input value={prefKey} onChange={(e) => setPrefKey(e.target.value)} placeholder="Preference key" className="border border-gray-200 rounded-xl px-3 py-2 text-xs" />
                      <input value={prefValue} onChange={(e) => setPrefValue(e.target.value)} placeholder="Preference value" className="border border-gray-200 rounded-xl px-3 py-2 text-xs" />
                    </div>
                    <button onClick={savePreference} className="text-[10px] font-black uppercase tracking-widest px-3 py-2 rounded-xl bg-black text-white">Save Preference</button>
                  </div>

                  <div className="space-y-2 mb-4">
                    <div className="grid grid-cols-3 gap-2">
                      <input value={memoryScope} onChange={(e) => setMemoryScope(e.target.value)} placeholder="scope" className="border border-gray-200 rounded-xl px-3 py-2 text-xs" />
                      <input value={memoryKey} onChange={(e) => setMemoryKey(e.target.value)} placeholder="memory key" className="border border-gray-200 rounded-xl px-3 py-2 text-xs" />
                      <input value={memoryValue} onChange={(e) => setMemoryValue(e.target.value)} placeholder="memory value" className="border border-gray-200 rounded-xl px-3 py-2 text-xs" />
                    </div>
                    <button onClick={saveMemory} className="text-[10px] font-black uppercase tracking-widest px-3 py-2 rounded-xl bg-black text-white">Save Memory</button>
                  </div>

                  <div className="max-h-44 overflow-y-auto space-y-2 no-scrollbar">
                    {preferences.map((p) => (
                      <div key={p.key} className="text-[11px] border border-gray-100 rounded-xl p-2">
                        <span className="font-black uppercase tracking-widest text-gray-500 mr-2">{p.key}</span>
                        <span className="text-gray-700">{p.value}</span>
                      </div>
                    ))}
                    {memories.slice(0, 8).map((m) => (
                      <div key={m.id} className="text-[11px] border border-gray-100 rounded-xl p-2">
                        <span className="font-black uppercase tracking-widest text-gray-500 mr-2">[{m.scope}] {m.memory_key}</span>
                        <span className="text-gray-700">{m.value}</span>
                      </div>
                    ))}
                  </div>
                </div>

                <div className="bg-white border border-[#E5E5E7] rounded-[2rem] p-6 shadow-sm">
                  <div className="flex items-center gap-2 mb-4">
                    <Brain className="w-4 h-4" />
                    <h3 className="text-xs font-black uppercase tracking-[0.3em] text-gray-500">Skill Studio</h3>
                  </div>

                  <div className="space-y-2 mb-4">
                    <input value={skillName} onChange={(e) => setSkillName(e.target.value)} placeholder="Skill name" className="w-full border border-gray-200 rounded-xl px-3 py-2 text-xs" />
                    <input value={skillDesc} onChange={(e) => setSkillDesc(e.target.value)} placeholder="Skill description" className="w-full border border-gray-200 rounded-xl px-3 py-2 text-xs" />
                    <textarea value={skillInstruction} onChange={(e) => setSkillInstruction(e.target.value)} placeholder="Skill instruction (what agent should do when this skill is active)" className="w-full border border-gray-200 rounded-xl px-3 py-2 text-xs min-h-[72px]" />
                    <button onClick={createSkill} className="text-[10px] font-black uppercase tracking-widest px-3 py-2 rounded-xl bg-black text-white">Create Skill</button>
                  </div>

                  <div className="max-h-44 overflow-y-auto space-y-2 no-scrollbar">
                    {skills.map((s) => (
                      <div key={s.id} className="text-[11px] border border-gray-100 rounded-xl p-2">
                        <div className="flex items-center justify-between gap-2">
                          <span className="font-black uppercase tracking-widest text-gray-600">{s.name}</span>
                          <div className="space-x-2">
                            <button onClick={() => toggleSkill(s.id, s.is_active)} className="text-[9px] px-2 py-1 rounded-full border border-gray-200">
                              {s.is_active === 1 ? 'Disable' : 'Enable'}
                            </button>
                            <button onClick={() => removeSkill(s.id)} className="text-[9px] px-2 py-1 rounded-full border border-rose-200 text-rose-600">
                              Delete
                            </button>
                          </div>
                        </div>
                        <div className="text-gray-700 mt-1">{s.description || s.instruction}</div>
                      </div>
                    ))}
                  </div>
                </div>
              </section>

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
                    <AgentCard name="Knowledge Agent" type="RAG Specialist" status="Ready" desc="Reads from local knowledge and vector context." />
                  </div>
                </div>

                <div className="space-y-6">
                  <h3 className="text-xs font-black uppercase tracking-[0.3em] text-gray-400">System Telemetry</h3>
                  <div className="bg-white border border-[#E5E5E7] rounded-3xl p-6 min-h-[300px] shadow-sm flex flex-col gap-4">
                    <TelemetryItem label="Inference Server" status={engineStatus === 'ACTIVE' ? 'Online' : 'Booting'} color={engineStatus === 'ACTIVE' ? 'emerald' : 'amber'} />
                    <TelemetryItem label="Knowledge Index" status="Standby" color="amber" />
                    <TelemetryItem label="Hermes Gateway" status={engineStatus === 'ACTIVE' ? 'Connected' : 'Waiting'} color={engineStatus === 'ACTIVE' ? 'emerald' : 'blue'} />
                    <div className="mt-auto pt-4 border-t border-dashed border-gray-100">
                      <button
                        onClick={() => setShowDiagnostics(true)}
                        className="text-[10px] font-black uppercase tracking-widest text-gray-400 hover:text-black transition-all flex items-center gap-2"
                      >
                        <Shield className="w-3 h-3" /> Open Diagnostics
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          ) : activeTab === 'neural' ? (
            <RagChat />
          ) : activeTab === 'builder' ? (
            <AgentBuilder />
          ) : (
            <div className="h-full overflow-y-auto p-8 no-scrollbar bg-gradient-to-b from-white/30 to-transparent space-y-6">
              <section className="bg-white border border-[#E5E5E7] rounded-[2rem] p-6 shadow-sm">
                <div className="flex items-center justify-between gap-3 mb-4">
                  <h3 className="text-xs font-black uppercase tracking-[0.3em] text-gray-500">Hermes Skills Hub</h3>
                  <button
                    onClick={() => refreshSkillMarketplace(skillSearch)}
                    className="text-[10px] font-black uppercase tracking-widest text-gray-400 hover:text-black"
                  >
                    Refresh
                  </button>
                </div>

                <div className="flex gap-2 mb-4">
                  <div className="flex-1 border border-gray-200 rounded-xl px-3 py-2 flex items-center gap-2">
                    <Search className="w-4 h-4 text-gray-400" />
                    <input
                      value={skillSearch}
                      onChange={(e) => setSkillSearch(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter') {
                          void refreshSkillMarketplace(skillSearch);
                        }
                      }}
                      placeholder="Search skills from Hermes docs"
                      className="w-full text-xs outline-none"
                    />
                  </div>
                  <button
                    onClick={() => refreshSkillMarketplace(skillSearch)}
                    className="text-[10px] font-black uppercase tracking-widest px-3 py-2 rounded-xl bg-black text-white"
                  >
                    Search
                  </button>
                </div>

                <div className="text-[10px] font-bold text-gray-500 uppercase tracking-widest mb-3">
                  Listed: {marketplaceSkills.length} skills
                </div>

                <div className="grid grid-cols-1 lg:grid-cols-2 gap-3 max-h-[60vh] overflow-y-auto no-scrollbar">
                  {marketplaceSkills.map((skill) => (
                    <div key={skill.name} className="border border-gray-100 rounded-[1.2rem] p-4 hover:border-black/10 transition-all">
                      <div className="flex items-center justify-between gap-2">
                        <div className="text-[13px] font-black tracking-tight">{skill.name}</div>
                        <div className="flex gap-1">
                          <span className={`text-[9px] px-2 py-1 rounded-full font-black uppercase tracking-widest ${skill.built_in ? 'bg-emerald-100 text-emerald-700' : 'bg-blue-100 text-blue-700'}`}>
                            {skill.built_in ? 'Built-in' : 'Community'}
                          </span>
                          <span className="text-[9px] px-2 py-1 rounded-full font-black uppercase tracking-widest bg-gray-100 text-gray-600">
                            {skill.registry}
                          </span>
                        </div>
                      </div>
                      <div className="text-[11px] text-gray-600 mt-2 leading-relaxed min-h-[34px]">
                        {skill.description || 'Hermes skill from the public skills catalog.'}
                      </div>
                      <div className="mt-3 flex justify-end">
                        <button
                          disabled={marketBusy === skill.name || skill.installed}
                          onClick={() => installMarketplaceSkill(skill.name)}
                          className="text-[10px] font-black uppercase tracking-widest px-3 py-2 rounded-xl bg-black text-white disabled:opacity-40"
                        >
                          {skill.installed ? 'Installed' : marketBusy === skill.name ? 'Installing...' : 'Download'}
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              </section>
            </div>
          )}
        </div>
      </main>

      {showConnect && <ConnectPanel onClose={() => setShowConnect(false)} />}
      {showDiagnostics && (
        <div className="fixed inset-0 z-50 pointer-events-none">
          <div className="absolute inset-0 bg-black/20 pointer-events-auto" onClick={() => setShowDiagnostics(false)} />
          <aside className="absolute top-0 right-0 h-full w-full max-w-[420px] bg-white border-l border-[#E5E5E7] shadow-2xl pointer-events-auto flex flex-col">
            <div className="p-6 border-b border-[#E5E5E7] bg-[#FAFAFA] flex items-center justify-between">
              <div>
                <div className="text-[10px] font-black uppercase tracking-[0.3em] text-gray-400">Sidebar Diagnostics</div>
                <h3 className="text-xl font-black text-[#1C1C1E] mt-1">Agent Health & Logs</h3>
              </div>
              <button onClick={() => setShowDiagnostics(false)} className="w-10 h-10 rounded-full border border-gray-200 flex items-center justify-center text-gray-500 hover:text-black hover:border-black/20 transition-all">
                <Plus className="w-4 h-4 rotate-45" />
              </button>
            </div>

            <div className="p-6 space-y-5 overflow-y-auto no-scrollbar flex-1">
              <div className="grid grid-cols-2 gap-3">
                <div className="border border-gray-100 rounded-[1.4rem] p-4 bg-white shadow-sm">
                  <div className="text-[10px] font-black uppercase tracking-[0.3em] text-gray-400">Health</div>
                  <div className="mt-2 text-lg font-black text-[#1C1C1E] capitalize">{agentHealth}</div>
                  <div className={`mt-3 inline-flex text-[9px] font-black uppercase tracking-widest px-2 py-1 rounded-full ${agentHealth === 'healthy' ? 'bg-emerald-50 border border-emerald-200 text-emerald-600' : agentHealth === 'busy' ? 'bg-amber-50 border border-amber-200 text-amber-600' : 'bg-rose-50 border border-rose-200 text-rose-600'}`}>
                    {agentHealth}
                  </div>
                </div>
                <div className="border border-gray-100 rounded-[1.4rem] p-4 bg-white shadow-sm">
                  <div className="text-[10px] font-black uppercase tracking-[0.3em] text-gray-400">Latency</div>
                  <div className="mt-2 text-lg font-black text-[#1C1C1E]">{lastResponseMs == null ? '—' : `${lastResponseMs} ms`}</div>
                  <div className="mt-3 text-[10px] text-gray-500 font-medium">Latest agent response time</div>
                </div>
              </div>

              <div className="border border-gray-100 rounded-[1.6rem] p-4 bg-white shadow-sm">
                <div className="flex items-center justify-between gap-3 mb-3">
                  <div>
                    <div className="text-[10px] font-black uppercase tracking-[0.3em] text-gray-400">Realtime Logs</div>
                    <div className="text-[11px] text-gray-500 mt-1">Agent execution, sync, and error events</div>
                  </div>
                  <div className="text-[9px] font-black uppercase tracking-widest px-2 py-1 rounded-full bg-gray-100 text-gray-600">{agentLogs.length} events</div>
                </div>
                <div className="space-y-2 max-h-[50vh] overflow-y-auto no-scrollbar pr-1">
                  {agentLogs.length === 0 ? (
                    <div className="text-[11px] text-gray-400 italic">Waiting for live agent activity...</div>
                  ) : (
                    agentLogs.map((log) => (
                      <div key={log.id} className="border border-gray-100 rounded-2xl p-3 bg-[#FAFAFA]">
                        <div className="flex items-center justify-between gap-3">
                          <div className="flex items-center gap-2 min-w-0">
                            <span className={`w-2 h-2 rounded-full ${log.level === 'error' ? 'bg-rose-500' : log.level === 'warning' ? 'bg-amber-500' : 'bg-emerald-500'}`} />
                            <span className="text-[10px] font-black uppercase tracking-widest text-gray-500 truncate">{log.scope}</span>
                          </div>
                          <span className="text-[9px] font-black uppercase tracking-widest text-gray-400 shrink-0">{log.ts}</span>
                        </div>
                        <div className="mt-2 text-[11px] font-medium text-[#1C1C1E] leading-relaxed">{log.message}</div>
                        {log.details && Object.keys(log.details).length > 0 && (
                          <div className="mt-2 text-[10px] text-gray-500 font-mono whitespace-pre-wrap break-words">
                            {JSON.stringify(log.details)}
                          </div>
                        )}
                      </div>
                    ))
                  )}
                </div>
              </div>
            </div>
          </aside>
        </div>
      )}
    </div>
  );
};

const CapChip: React.FC<{ label: string; enabled: boolean }> = ({ label, enabled }) => (
  <span className={`text-[9px] font-black uppercase tracking-widest px-2 py-1 rounded-full ${enabled ? 'bg-emerald-100 text-emerald-700' : 'bg-gray-100 text-gray-500'}`}>
    {label}
  </span>
);

const StatCard: React.FC<{ icon: React.ReactNode; label: string; val: string; sub: string }> = ({ icon, label, val, sub }) => (
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

const AgentCard: React.FC<{ name: string; type: string; status: string; desc: string }> = ({ name, type: agentType, status, desc }) => (
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

const TelemetryItem: React.FC<{ label: string; status: string; color: string }> = ({ label, status, color }) => {
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
