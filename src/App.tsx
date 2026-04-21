import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import LoadingScreen from './components/Onboarding/LoadingScreen';
import AuthScreen from './components/Onboarding/AuthScreen';
import RunModelChoice from './components/Onboarding/RunModelChoice';
import ModelPicker from './components/Onboarding/ModelPicker';
import DownloadScreen from './components/Onboarding/DownloadScreen';
import Dashboard from './components/Dashboard';

import './App.css';

function App() {
  const [step, setStep] = useState<number>(0); 
  const [error, setError] = useState<string | null>(null);
  const [bootRoute, setBootRoute] = useState<'loading' | 'setup' | 'dashboard'>('loading');

  // Initialize data folders on mount
  useEffect(() => {
    const initFolders = async () => {
      try {
        console.log("[App] Initializing folders...");
        const path = await invoke<string>('ensure_data_folder');
        console.log("✅ Operarius folders initialized at:", path);
      } catch (err) {
        console.error("Folder initialization failed:", err);
        setError(String(err));
      }
    };
    initFolders();
  }, []);

  useEffect(() => {
    const hydrateLaunchState = async () => {
      try {
        const completed = await invoke<boolean>('get_app_flag', { key: 'setup_complete' });
        if (completed) {
          setBootRoute('dashboard');
          setStep(6);
        } else {
          setBootRoute('setup');
          setStep(0);
        }
      } catch (err) {
        console.error('Failed to read app launch state:', err);
        setBootRoute('setup');
        setStep(0);
      }
    };

    hydrateLaunchState();
  }, []);

  // Auto-transition from Loading (0) to Auth (1) after 2s
  useEffect(() => {
    if (bootRoute !== 'setup') {
      return;
    }

    if (step === 0) {
      const timer = setTimeout(() => setStep(1), 2000);
      return () => clearTimeout(timer);
    }
  }, [step, bootRoute]);

  useEffect(() => {
    if (step !== 6) {
      return;
    }

    invoke('set_app_flag', { key: 'setup_complete', value: 'true' }).catch((err) => {
      console.error('Failed to persist setup completion:', err);
    });
  }, [step]);

  const openSetupFlow = () => {
    setBootRoute('setup');
    setStep(0);
  };

  if (error) {
    return (
      <div className="h-screen w-screen flex items-center justify-center bg-white p-10 text-center">
        <div className="max-w-md">
          <h1 className="text-xl font-bold text-red-600 mb-4">Crush Protocol Active</h1>
          <p className="text-gray-500 text-sm mb-6">{error}</p>
          <button onClick={() => window.location.reload()} className="bg-black text-white px-6 py-2 rounded-lg text-xs font-bold uppercase">Re-Ignite App</button>
        </div>
      </div>
    );
  }

  return (
    <div className="antialiased text-[#1C1C1E] selection:bg-black selection:text-white h-full w-full">
      {step === 0 && <LoadingScreen onRetry={() => setStep(1)} />}
      {step === 1 && <AuthScreen onComplete={() => setStep(2)} />}
      {step === 2 && <RunModelChoice onSelectLocal={() => setStep(3)} />}
      {step === 3 && <ModelPicker onContinue={() => setStep(4)} />}
      {step === 4 && <DownloadScreen onComplete={() => setStep(6)} />}
      {step === 6 && <Dashboard onOpenSetup={openSetupFlow} />}
    </div>
  );
}

export default App;
