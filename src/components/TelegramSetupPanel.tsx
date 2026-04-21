import React, { useState } from 'react';
import { X, Check, AlertCircle, Copy, ExternalLink } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

interface TelegramSetupPanelProps {
  onClose: () => void;
  onSuccess?: () => void;
}

export const TelegramSetupPanel: React.FC<TelegramSetupPanelProps> = ({ onClose, onSuccess }) => {
  const [step, setStep] = useState<'guide' | 'input' | 'success' | 'error'>('guide');
  const [token, setToken] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [copied, setCopied] = useState(false);

  const botFatherLink = 'https://t.me/BotFather';
  const userinfoLink = 'https://t.me/userinfobot';

  const shellClass = 'bg-white border border-[#E5E5E7] rounded-[2rem] shadow-2xl';
  const titleClass = 'text-xs font-black uppercase tracking-[0.3em] text-gray-500';
  const primaryButton = 'px-5 py-3 rounded-xl bg-black text-white text-[10px] font-black uppercase tracking-widest hover:scale-[1.01] active:scale-[0.99] transition-all';
  const secondaryButton = 'px-5 py-3 rounded-xl border border-gray-200 text-gray-600 text-[10px] font-black uppercase tracking-widest hover:border-black/20 hover:text-black transition-all';

  const handleCopyLink = () => {
    navigator.clipboard.writeText(botFatherLink);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleSetupTelegram = async () => {
    if (!token.trim()) {
      setError('Please enter your bot token');
      return;
    }

    setLoading(true);
    setError('');

    try {
      await invoke('setup_telegram_bot', { botToken: token });
      setStep('success');
      setTimeout(() => {
        if (onSuccess) onSuccess();
        onClose();
      }, 3000);
    } catch (err) {
      setStep('error');
      setError(String(err) || 'Failed to configure Telegram bot');
      setLoading(false);
    }
  };

  if (step === 'guide') {
    return (
      <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
        <div className={`${shellClass} max-w-2xl w-full max-h-[90vh] overflow-hidden flex flex-col`}>
          <div className="p-6 border-b border-[#E5E5E7] bg-[#FAFAFA] flex items-center justify-between">
            <div>
              <div className={titleClass}>Telegram Connect</div>
              <h2 className="text-2xl font-black tracking-tight text-[#1C1C1E] mt-1">Setup Telegram Bot</h2>
            </div>
            <button onClick={onClose} className="w-10 h-10 rounded-full border border-gray-200 flex items-center justify-center text-gray-500 hover:text-black hover:border-black/20 transition-all">
              <X className="w-4 h-4" />
            </button>
          </div>

          <div className="p-6 space-y-4 overflow-y-auto no-scrollbar">
            <div className="grid gap-4 md:grid-cols-3">
              {[
                ['1', 'Create bot', 'Open @BotFather and create a new bot.'],
                ['2', 'Copy token', 'Paste the bot token here after BotFather gives it to you.'],
                ['3', 'Connect', 'Operarius will link Telegram to your local agent.'],
              ].map(([stepNumber, title, copy]) => (
                <div key={stepNumber} className="border border-gray-100 rounded-[1.4rem] p-4 bg-white shadow-sm">
                  <div className="w-8 h-8 rounded-full bg-black text-white flex items-center justify-center text-[10px] font-black mb-3">{stepNumber}</div>
                  <div className="text-sm font-black text-[#1C1C1E]">{title}</div>
                  <div className="text-[11px] text-gray-500 leading-relaxed mt-2">{copy}</div>
                </div>
              ))}
            </div>

            <div className="border border-gray-100 rounded-[1.6rem] p-5 bg-white shadow-sm">
              <div className="flex items-start gap-3">
                <AlertCircle className="w-5 h-5 text-amber-500 shrink-0 mt-0.5" />
                <div>
                  <div className="text-sm font-black text-[#1C1C1E]">Keep your token private</div>
                  <p className="text-[11px] text-gray-500 mt-1 leading-relaxed">
                    Never share your bot token publicly. Anyone with the token can control your bot.
                  </p>
                </div>
              </div>
            </div>

            <div className="space-y-2">
              <div className="text-[10px] font-black uppercase tracking-[0.3em] text-gray-400">Quick Links</div>
              <div className="flex flex-wrap gap-2">
                <a href={botFatherLink} target="_blank" rel="noopener noreferrer" className={primaryButton}>
                  Open @BotFather <ExternalLink className="w-3.5 h-3.5 inline-block ml-1" />
                </a>
                <button onClick={handleCopyLink} className={secondaryButton}>
                  <Copy className="w-3.5 h-3.5 inline-block mr-1" />
                  {copied ? 'Copied' : 'Copy Link'}
                </button>
                <a href={userinfoLink} target="_blank" rel="noopener noreferrer" className={secondaryButton}>
                  Verify Account
                </a>
              </div>
            </div>

            <div className="flex gap-3 pt-2">
              <button onClick={() => setStep('input')} className={primaryButton}>
                I've Got My Token
              </button>
              <button onClick={onClose} className={secondaryButton}>
                Cancel
              </button>
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (step === 'input') {
    return (
      <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
        <div className={`${shellClass} max-w-md w-full overflow-hidden`}>
          <div className="p-6 border-b border-[#E5E5E7] bg-[#FAFAFA] flex justify-between items-center">
            <div>
              <div className={titleClass}>Telegram Connect</div>
              <h2 className="text-xl font-black tracking-tight text-[#1C1C1E] mt-1">Paste Bot Token</h2>
            </div>
            <button onClick={onClose} className="w-10 h-10 rounded-full border border-gray-200 flex items-center justify-center text-gray-500 hover:text-black hover:border-black/20 transition-all">
              <X className="w-4 h-4" />
            </button>
          </div>

          <div className="p-6 space-y-4">
            <div>
              <label className="block text-[10px] font-black uppercase tracking-[0.3em] text-gray-400 mb-2">
                Bot Token from @BotFather
              </label>
              <input
                type="password"
                value={token}
                onChange={(e) => {
                  setToken(e.target.value);
                  setError('');
                }}
                placeholder="123456789:ABCdefGHIjklMNOpqrSTUvwxYZ"
                className="w-full px-4 py-3 border border-gray-200 rounded-2xl focus:outline-none focus:border-black/20 bg-white font-mono text-sm"
              />
              <p className="text-[11px] text-gray-500 mt-2">Token is hidden for security. Paste from your clipboard.</p>
            </div>

            {error && (
              <div className="bg-rose-50 border border-rose-200 text-rose-700 px-4 py-3 rounded-2xl text-sm">
                {error}
              </div>
            )}

            <div className="flex gap-3 pt-2">
              <button
                onClick={handleSetupTelegram}
                disabled={loading || !token.trim()}
                className={`${primaryButton} flex-1 disabled:opacity-40 disabled:cursor-not-allowed`}
              >
                {loading ? 'Configuring...' : 'Configure Telegram'}
              </button>
              <button
                onClick={() => setStep('guide')}
                disabled={loading}
                className={`${secondaryButton} disabled:opacity-50`}
              >
                Back
              </button>
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (step === 'success') {
    return (
      <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
        <div className="bg-white rounded-2xl shadow-2xl max-w-md w-full p-8">
          <div className="flex justify-center mb-4">
            <div className="w-16 h-16 bg-green-100 rounded-full flex items-center justify-center">
              <Check className="w-8 h-8 text-green-600" />
            </div>
          </div>
          <h2 className="text-2xl font-bold text-center text-gray-900 mb-2">
            Telegram Configured!
          </h2>
          <p className="text-center text-gray-600 text-sm mb-4">
            Your Telegram bot is now connected to Operarius. You can chat with it on Telegram!
          </p>
          <p className="text-center text-gray-500 text-xs mb-6">
            Closing in 3 seconds...
          </p>
          <button
            onClick={onClose}
            className="w-full px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-all font-medium"
          >
            Close
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
      <div className="bg-white rounded-2xl shadow-2xl max-w-md w-full p-8">
        <div className="flex justify-center mb-4">
          <div className="w-16 h-16 bg-red-100 rounded-full flex items-center justify-center">
            <AlertCircle className="w-8 h-8 text-red-600" />
          </div>
        </div>
        <h2 className="text-2xl font-bold text-center text-gray-900 mb-2">
          Configuration Failed
        </h2>
        <p className="text-center text-gray-600 text-sm mb-6">
          {error || 'An error occurred while configuring Telegram.'}
        </p>
        <div className="flex gap-3">
          <button
            onClick={() => setStep('input')}
            className="flex-1 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-all font-medium text-sm"
          >
            Try Again
          </button>
          <button
            onClick={onClose}
            className="flex-1 px-4 py-2 bg-gray-200 text-gray-800 rounded-lg hover:bg-gray-300 transition-all font-medium text-sm"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
};
