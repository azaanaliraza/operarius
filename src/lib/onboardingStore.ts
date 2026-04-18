import { create } from 'zustand';

export type OnboardingStep = 
  | 'loading' 
  | 'auth' 
  | 'run-model-choice' 
  | 'model-picker' 
  | 'download' 
  | 'hermes-setup' 
  | 'dashboard';

interface OnboardingState {
  step: OnboardingStep;
  setStep: (step: OnboardingStep) => void;
  selectedModel: string | null;
  modelRepo: string | null;
  modelFile: string | null;
  setSelectedModel: (model: string | null, repo: string | null, file: string | null) => void;
  runType: 'local' | 'cloud' | null;
  setRunType: (type: 'local' | 'cloud' | null) => void;
}

export const useOnboardingStore = create<OnboardingState>((set) => ({
  step: 'loading',
  setStep: (step) => set({ step }),
  selectedModel: null,
  modelRepo: null,
  modelFile: null,
  setSelectedModel: (model, repo, file) => set({ selectedModel: model, modelRepo: repo, modelFile: file }),
  runType: null,
  setRunType: (runType) => set({ runType }),
}));
