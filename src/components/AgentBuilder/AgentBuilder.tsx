import React, { useCallback, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Background,
  Connection,
  Controls,
  MiniMap,
  ReactFlow,
  ReactFlowProvider,
  addEdge,
  useEdgesState,
  useNodesState,
  type Edge,
  type Node,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import {
  Bot,
  Brain,
  Database,
  Play,
  Save,
  Share2,
  Sparkles,
  Timer,
  Trash2,
  Wand2,
} from 'lucide-react';

type BuilderNodeType =
  | 'trigger'
  | 'agent'
  | 'model'
  | 'memory'
  | 'knowledge'
  | 'tool'
  | 'logic';

interface BuilderNodeData {
  label: string;
  kind: BuilderNodeType;
  description: string;
  status: 'idle' | 'running' | 'success' | 'error';
  config: Record<string, string | number | boolean>;
}

interface PaletteItem {
  kind: BuilderNodeType;
  label: string;
  description: string;
}

const palette: PaletteItem[] = [
  { kind: 'trigger', label: 'Chat Trigger', description: 'Entry point for user prompts' },
  { kind: 'agent', label: 'Hermes Agent', description: 'Reasoning and planning core' },
  { kind: 'model', label: 'Local LLM', description: 'Llama model runtime' },
  { kind: 'memory', label: 'SQLite Memory', description: 'Conversation context' },
  { kind: 'knowledge', label: 'RAG Retriever', description: 'Knowledge lookup' },
  { kind: 'tool', label: 'Telegram Tool', description: 'Outbound actions' },
  { kind: 'logic', label: 'IF Branch', description: 'Conditional routing' },
];

const defaultConfigByKind: Record<BuilderNodeType, Record<string, string | number | boolean>> = {
  trigger: { source: 'chat', throttle_ms: 0 },
  agent: { system_prompt: 'You are a precise local assistant.', max_iterations: 8, keep_alive: true },
  model: { runtime: 'llama.cpp', temperature: 0.6, context_window: 65536 },
  memory: { type: 'sqlite', max_history: 50 },
  knowledge: { max_results: 3, threshold: 0.7 },
  tool: { provider: 'telegram', enabled: true },
  logic: { expression: 'contains(intent, "support")' },
};

const makeNode = (kind: BuilderNodeType, x: number, y: number): Node<BuilderNodeData> => ({
  id: `${kind}-${Date.now()}-${Math.random().toString(16).slice(2, 8)}`,
  type: 'default',
  position: { x, y },
  data: {
    label: palette.find((p) => p.kind === kind)?.label || kind,
    kind,
    description: palette.find((p) => p.kind === kind)?.description || '',
    status: 'idle',
    config: { ...defaultConfigByKind[kind] },
  },
});

const initialNodes: Node<BuilderNodeData>[] = [
  makeNode('trigger', 120, 100),
  makeNode('agent', 430, 100),
  makeNode('model', 430, 260),
  makeNode('memory', 430, 420),
  makeNode('tool', 760, 100),
];

const initialEdges: Edge[] = [
  { id: 'e-trigger-agent', source: initialNodes[0].id, target: initialNodes[1].id, animated: true },
  { id: 'e-agent-tool', source: initialNodes[1].id, target: initialNodes[4].id },
  { id: 'e-model-agent', source: initialNodes[2].id, target: initialNodes[1].id },
  { id: 'e-memory-agent', source: initialNodes[3].id, target: initialNodes[1].id },
];

function AgentBuilderInner() {
  const [nodes, setNodes, onNodesChange] = useNodesState<BuilderNodeData>(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(initialNodes[1]?.id || null);
  const [runState, setRunState] = useState<'idle' | 'running'>('idle');
  const [workflowName, setWorkflowName] = useState('Telegram Q&A Agent');
  const [statusNote, setStatusNote] = useState('Ready for multi-agent automation.');
  const canvasRef = useRef<HTMLDivElement>(null);

  const selectedNode = useMemo(
    () => nodes.find((n) => n.id === selectedNodeId) || null,
    [nodes, selectedNodeId]
  );

  const onConnect = useCallback((connection: Connection) => {
    if (!connection.source || !connection.target) {
      return;
    }

    const source = nodes.find((n) => n.id === connection.source);
    const target = nodes.find((n) => n.id === connection.target);
    if (!source || !target) {
      return;
    }

    // Keep validation light but meaningful: agent can connect to everything,
    // triggers and logic cannot feed model/memory directly.
    const invalid =
      source.data.kind === 'trigger' && ['model', 'memory'].includes(target.data.kind);

    if (invalid) {
      return;
    }

    setEdges((current) =>
      addEdge({ ...connection, id: `e-${connection.source}-${connection.target}-${Date.now()}` }, current)
    );
  }, [nodes, setEdges]);

  const addNodeToCanvas = (kind: BuilderNodeType) => {
    const bounds = canvasRef.current?.getBoundingClientRect();
    const x = (bounds?.width || 1200) * 0.5 + (Math.random() * 120 - 60);
    const y = (bounds?.height || 700) * 0.35 + (Math.random() * 120 - 60);
    const next = makeNode(kind, x, y);
    setNodes((current) => [...current, next]);
    setSelectedNodeId(next.id);
  };

  const removeSelectedNode = () => {
    if (!selectedNodeId) return;
    setNodes((current) => current.filter((n) => n.id !== selectedNodeId));
    setEdges((current) => current.filter((e) => e.source !== selectedNodeId && e.target !== selectedNodeId));
    setSelectedNodeId(null);
  };

  const updateSelectedConfig = (key: string, value: string) => {
    if (!selectedNodeId) return;

    setNodes((current) =>
      current.map((node) =>
        node.id === selectedNodeId
          ? {
              ...node,
              data: {
                ...node.data,
                config: {
                  ...node.data.config,
                  [key]: value,
                },
              },
            }
          : node
      )
    );
  };

  const toWorkflowPayload = () => ({
    id: `workflow-${Date.now()}`,
    name: workflowName,
    nodes: nodes.map((node) => ({
      id: node.id,
      node_type: node.type,
      data: node.data,
    })),
    edges: edges.map((edge) => ({
      id: edge.id,
      source: edge.source,
      target: edge.target,
    })),
  });

  const runWorkflow = async () => {
    setRunState('running');
    try {
      const payload = toWorkflowPayload();
      const compiled = await invoke<{
        workflow_id: string;
        execution_order: string[];
        warnings: string[];
        capability_flags: string[];
      }>('compile_agent_workflow', {
        workflowJson: payload,
      });

      for (const id of compiled.execution_order) {
        setNodes((current) =>
          current.map((n) =>
            n.id === id
              ? {
                  ...n,
                  data: { ...n.data, status: 'running' },
                }
              : n
          )
        );

        await new Promise((resolve) => setTimeout(resolve, 180));

        setNodes((current) =>
          current.map((n) =>
            n.id === id
              ? {
                  ...n,
                  data: { ...n.data, status: 'success' },
                }
              : n
          )
        );
      }

      const warningText = compiled.warnings.length > 0
        ? ` | warnings: ${compiled.warnings.length}`
        : '';
      setStatusNote(`Compiled ${compiled.execution_order.length} steps (${compiled.capability_flags.join(', ')})${warningText}`);
    } catch (error) {
      console.error('Workflow compile failed:', error);
      setStatusNote(`Compile failed: ${String(error)}`);
      setNodes((current) => current.map((n) => ({ ...n, data: { ...n.data, status: 'error' } })));
    } finally {
      setRunState('idle');
    }
  };

  const saveWorkflow = async () => {
    const payload = {
      id: `workflow-${Date.now()}`,
      name: workflowName,
      updated_at: new Date().toISOString(),
      nodes: nodes.map((node) => ({
        id: node.id,
        node_type: node.type,
        data: node.data,
      })),
      edges: edges.map((edge) => ({
        id: edge.id,
        source: edge.source,
        target: edge.target,
      })),
    };
    try {
      const savedId = await invoke<string>('save_agent_workflow', {
        userId: 'local-user',
        name: workflowName,
        workflowJson: payload,
      });
      setStatusNote(`Saved workflow ${savedId}`);
    } catch (error) {
      localStorage.setItem('operarius-agent-builder-workflow', JSON.stringify(payload));
      setStatusNote(`Saved locally (backend unavailable): ${String(error)}`);
    }
  };

  const loadWorkflow = async () => {
    try {
      const records = await invoke<Array<{ name: string; workflow_json: { nodes: Node<BuilderNodeData>[]; edges: Edge[] } }>>('list_agent_workflows', {
        userId: 'local-user',
      });

      if (records.length > 0) {
        const parsed = records[0].workflow_json;
        setWorkflowName(records[0].name || 'Loaded Workflow');
        setNodes(parsed.nodes || []);
        setEdges(parsed.edges || []);
        setSelectedNodeId(parsed.nodes?.[0]?.id || null);
        setStatusNote('Loaded latest saved workflow from vault.');
        return;
      }

      const raw = localStorage.getItem('operarius-agent-builder-workflow');
      if (!raw) return;
      const parsed = JSON.parse(raw) as { name: string; nodes: Node<BuilderNodeData>[]; edges: Edge[] };
      setWorkflowName(parsed.name || 'Loaded Workflow');
      setNodes(parsed.nodes || []);
      setEdges(parsed.edges || []);
      setSelectedNodeId(parsed.nodes?.[0]?.id || null);
      setStatusNote('Loaded local backup workflow.');
    } catch (error) {
      console.error('Failed to load workflow:', error);
      setStatusNote(`Load failed: ${String(error)}`);
    }
  };

  const statusClass = (status: BuilderNodeData['status']) => {
    if (status === 'running') return 'bg-amber-100 text-amber-700 border-amber-200';
    if (status === 'success') return 'bg-emerald-100 text-emerald-700 border-emerald-200';
    if (status === 'error') return 'bg-rose-100 text-rose-700 border-rose-200';
    return 'bg-gray-100 text-gray-600 border-gray-200';
  };

  return (
    <div className="h-full overflow-hidden bg-gradient-to-b from-white/40 to-transparent p-6">
      <div className="h-full rounded-[2rem] border border-[#E5E5E7] bg-white shadow-sm overflow-hidden grid grid-cols-[280px_1fr_320px]">
        <aside className="border-r border-[#E5E5E7] p-5 overflow-y-auto no-scrollbar">
          <div className="text-[10px] font-black uppercase tracking-[0.3em] text-gray-400">Agent Builder</div>
          <div className="mt-2 text-[10px] text-gray-500 leading-relaxed">{statusNote}</div>
          <input
            value={workflowName}
            onChange={(event) => setWorkflowName(event.target.value)}
            className="mt-3 w-full rounded-xl border border-gray-200 px-3 py-2 text-xs font-semibold text-[#1C1C1E] outline-none"
          />

          <div className="mt-6 space-y-2">
            {palette.map((item) => (
              <button
                key={item.kind}
                onClick={() => addNodeToCanvas(item.kind)}
                className="w-full rounded-2xl border border-gray-100 px-3 py-3 text-left hover:border-black/20 transition-all"
              >
                <div className="text-[11px] font-black uppercase tracking-widest text-[#1C1C1E]">{item.label}</div>
                <div className="mt-1 text-[11px] text-gray-500 leading-relaxed">{item.description}</div>
              </button>
            ))}
          </div>

          <div className="mt-6 grid grid-cols-2 gap-2">
            <button
              onClick={runWorkflow}
              disabled={runState === 'running'}
              className="rounded-xl bg-black text-white text-[10px] font-black uppercase tracking-widest py-2.5 disabled:opacity-50"
            >
              <Play className="w-3 h-3 inline-block mr-1" /> Run
            </button>
            <button
              onClick={saveWorkflow}
              className="rounded-xl border border-gray-200 text-[10px] font-black uppercase tracking-widest py-2.5"
            >
              <Save className="w-3 h-3 inline-block mr-1" /> Save
            </button>
            <button
              onClick={loadWorkflow}
              className="rounded-xl border border-gray-200 text-[10px] font-black uppercase tracking-widest py-2.5 col-span-2"
            >
              <Timer className="w-3 h-3 inline-block mr-1" /> Load Last Saved
            </button>
          </div>
        </aside>

        <section ref={canvasRef} className="relative">
          <ReactFlow
            nodes={nodes}
            edges={edges}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            onConnect={onConnect}
            onNodeClick={(_, node) => setSelectedNodeId(node.id)}
            fitView
          >
            <Background gap={18} size={1} color="#E5E7EB" />
            <MiniMap zoomable pannable className="!bg-white !border !border-gray-200 !rounded-xl" />
            <Controls className="!bg-white !border !border-gray-200 !rounded-xl" />
          </ReactFlow>

          <div className="absolute top-4 left-4 rounded-full border border-gray-200 bg-white px-3 py-1.5 text-[9px] font-black uppercase tracking-widest text-gray-500">
            {nodes.length} nodes • {edges.length} edges
          </div>
        </section>

        <aside className="border-l border-[#E5E5E7] p-5 overflow-y-auto no-scrollbar">
          <div className="flex items-center justify-between gap-2">
            <div>
              <div className="text-[10px] font-black uppercase tracking-[0.3em] text-gray-400">Node Inspector</div>
              <div className="text-sm font-black text-[#1C1C1E] mt-1">{selectedNode?.data.label || 'Select a node'}</div>
            </div>
            <button
              onClick={removeSelectedNode}
              disabled={!selectedNode}
              className="rounded-xl border border-rose-200 text-rose-600 px-2.5 py-1.5 text-[10px] font-black uppercase tracking-widest disabled:opacity-40"
            >
              <Trash2 className="w-3 h-3 inline-block mr-1" /> Delete
            </button>
          </div>

          {selectedNode ? (
            <>
              <div className="mt-4 rounded-2xl border border-gray-100 p-3">
                <div className="text-[10px] font-black uppercase tracking-widest text-gray-500">Status</div>
                <span className={`mt-2 inline-flex rounded-full border px-2 py-1 text-[9px] font-black uppercase tracking-widest ${statusClass(selectedNode.data.status)}`}>
                  {selectedNode.data.status}
                </span>
                <div className="mt-3 text-[11px] text-gray-500 leading-relaxed">{selectedNode.data.description}</div>
              </div>

              <div className="mt-4 space-y-3">
                {Object.entries(selectedNode.data.config).map(([key, value]) => (
                  <label key={key} className="block">
                    <div className="text-[10px] font-black uppercase tracking-widest text-gray-400 mb-1">{key}</div>
                    <input
                      value={String(value)}
                      onChange={(event) => updateSelectedConfig(key, event.target.value)}
                      className="w-full rounded-xl border border-gray-200 px-3 py-2 text-xs outline-none"
                    />
                  </label>
                ))}
              </div>

              <div className="mt-6 rounded-2xl border border-gray-100 p-3 bg-[#FAFAFA]">
                <div className="text-[10px] font-black uppercase tracking-widest text-gray-400">Connection Hints</div>
                <ul className="mt-2 space-y-1 text-[11px] text-gray-600">
                  <li className="flex items-center gap-2"><Bot className="w-3 h-3" /> Agent should connect to Model, Memory, and Tool nodes.</li>
                  <li className="flex items-center gap-2"><Database className="w-3 h-3" /> Knowledge node improves factual responses.</li>
                  <li className="flex items-center gap-2"><Brain className="w-3 h-3" /> Keep context near 65536 for local models.</li>
                </ul>
              </div>
            </>
          ) : (
            <div className="mt-4 rounded-2xl border border-dashed border-gray-200 p-5 text-[11px] text-gray-500 leading-relaxed">
              Select a node to edit behavior and execution settings.
            </div>
          )}

          <div className="mt-6 border-t border-dashed border-gray-200 pt-4">
            <div className="text-[10px] font-black uppercase tracking-widest text-gray-400">Templates</div>
            <div className="mt-2 grid grid-cols-1 gap-2">
              <button
                onClick={() => {
                  setNodes([
                    makeNode('trigger', 120, 160),
                    makeNode('agent', 450, 160),
                    makeNode('model', 450, 360),
                    makeNode('memory', 700, 360),
                    makeNode('knowledge', 700, 160),
                    makeNode('tool', 780, 80),
                  ]);
                  setEdges([]);
                  setStatusNote('Loaded Telegram Assistant template.');
                }}
                className="rounded-xl border border-gray-200 px-3 py-2 text-left"
              >
                <div className="text-[10px] font-black uppercase tracking-widest text-[#1C1C1E]"><Wand2 className="w-3 h-3 inline-block mr-1" /> Telegram Assistant</div>
                <div className="mt-1 text-[10px] text-gray-500">Trigger + Agent + RAG + Telegram tool</div>
              </button>
              <button
                onClick={() => {
                  setNodes([makeNode('trigger', 120, 220), makeNode('agent', 460, 220), makeNode('logic', 780, 220)]);
                  setEdges([]);
                  setStatusNote('Loaded Intent Router template.');
                }}
                className="rounded-xl border border-gray-200 px-3 py-2 text-left"
              >
                <div className="text-[10px] font-black uppercase tracking-widest text-[#1C1C1E]"><Share2 className="w-3 h-3 inline-block mr-1" /> Intent Router</div>
                <div className="mt-1 text-[10px] text-gray-500">Route requests based on detected intent</div>
              </button>
              <button
                onClick={() => {
                  const n1 = makeNode('trigger', 80, 180);
                  n1.data.label = 'Inbox Trigger';
                  n1.data.config = { source: 'gmail' };

                  const n2 = makeNode('agent', 350, 180);
                  n2.data.label = 'OpenClaw Orchestrator';

                  const n3 = makeNode('tool', 620, 80);
                  n3.data.label = 'Slides Summarizer';
                  n3.data.config = { provider: 'google_slides', mode: 'summary' };

                  const n4 = makeNode('tool', 620, 180);
                  n4.data.label = 'Jira + GA4 + GSC';
                  n4.data.config = { provider: 'jira_ga4_gsc', mode: 'seo_ops' };

                  const n5 = makeNode('tool', 620, 280);
                  n5.data.label = 'Slack Webhook';
                  n5.data.config = { provider: 'slack_webhook', enabled: true };

                  const n6 = makeNode('knowledge', 350, 360);
                  n6.data.label = 'Zendesk + Docs RAG';
                  n6.data.config = { source: 'zendesk_docs', max_results: 3 };

                  setNodes([n1, n2, n3, n4, n5, n6]);
                  setEdges([
                    { id: 'oc-1', source: n1.id, target: n2.id },
                    { id: 'oc-2', source: n2.id, target: n3.id },
                    { id: 'oc-3', source: n2.id, target: n4.id },
                    { id: 'oc-4', source: n2.id, target: n5.id },
                    { id: 'oc-5', source: n6.id, target: n2.id },
                  ]);
                  setStatusNote('Loaded OpenClaw Army template for inbox, docs, SEO, and support automation.');
                }}
                className="rounded-xl border border-gray-200 px-3 py-2 text-left"
              >
                <div className="text-[10px] font-black uppercase tracking-widest text-[#1C1C1E]"><Sparkles className="w-3 h-3 inline-block mr-1" /> OpenClaw Army Ops</div>
                <div className="mt-1 text-[10px] text-gray-500">Email triage + slide review + SEO + support bot stack</div>
              </button>
            </div>
          </div>
        </aside>
      </div>
    </div>
  );
}

export default function AgentBuilder() {
  return (
    <ReactFlowProvider>
      <AgentBuilderInner />
    </ReactFlowProvider>
  );
}
