import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import LoadingScreen from './components/Onboarding/LoadingScreen';
import AuthScreen from './components/Onboarding/AuthScreen';
import RunModelChoice from './components/Onboarding/RunModelChoice';
import ModelPicker from './components/Onboarding/ModelPicker';
import DownloadScreen from './components/Onboarding/DownloadScreen';
import HermesSetup from './components/Onboarding/HermesSetup';
import Dashboard from './components/Dashboard';
import './App.css';

function App() {
  const [step, setStep] = useState(0); // 0=loading, 1=auth, 2=choice, 3=model picker, 4=download, 5=hermes, 6=dashboard

  const next = () => setStep(s => s + 1);

  // Initialize data folders on mount
  useEffect(() => {
    const initFolders = async () => {
      try {
        const path = await invoke<string>('ensure_data_folder');
        console.log("✅ Operarius folders initialized at:", path);
      } catch (err) {
        console.error("Folder initialization failed:", err);
      }
    };
    initFolders();
  }, []);

  // Auto-transition from Loading to Auth for demo purposes
  useEffect(() => {
    if (step === 0) {
      const timer = setTimeout(next, 3000);
      return () => clearTimeout(timer);
    }
  }, [step]);

  return (
    <div className="antialiased text-[#1C1C1E] selection:bg-black selection:text-white">
      {step === 0 && (
        <LoadingScreen onRetry={next} />
      )}
      
      {step === 1 && (
        <AuthScreen onComplete={next} />
      )}
      
      {step === 2 && (
        <RunModelChoice onSelectLocal={next} />
      )}
      
      {step === 3 && (
        <ModelPicker onContinue={next} />
      )}
      
      {step === 4 && (
        <DownloadScreen onComplete={next} />
      )}
      
      {step === 5 && (
        <HermesSetup onComplete={next} />
      )}
      
      {step === 6 && <Dashboard />}
    </div>
  );
}

export default App;
