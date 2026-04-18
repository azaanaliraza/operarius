import { create } from 'zustand';
export const useAppStore = create(() => ({ agents: [], currentAgent: null }));
