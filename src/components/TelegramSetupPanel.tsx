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
        <div className="bg-white rounded-2xl shadow-2xl max-w-2xl w-full max-h-[90vh] overflow-auto">
          {/* Header */}
          <div className="sticky top-0 bg-gradient-to-r from-blue-600 to-blue-700 text-white p-6 flex justify-between items-center">
            <h2 className="text-2xl font-bold">Setup Telegram Bot</h2>
            <button
              onClick={onClose}
              className="p-2 hover:bg-white/20 rounded-lg transition-all"
            >
              <X className="w-5 h-5" />
            </button>
          </div>

          {/* Content */}
          <div className="p-8 space-y-6">
            {/* Step 1 */}
            <div className="space-y-3">
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 bg-blue-100 text-blue-600 rounded-full flex items-center justify-center font-bold text-sm">
                  1
                </div>
                <h3 className="text-lg font-semibold text-gray-900">Create a Bot via @BotFather</h3>
              </div>
              <p className="text-sm text-gray-600 ml-11">
                Telegram requires all bots to be created through @BotFather, an official bot management tool.
              </p>
              <div className="ml-11 flex gap-2">
                <a
                  href={botFatherLink}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="inline-flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-all text-sm font-medium"
                >
                  Open @BotFather
                  <ExternalLink className="w-4 h-4" />
                </a>
                <button
                  onClick={handleCopyLink}
                  className="inline-flex items-center gap-2 px-4 py-2 bg-gray-200 text-gray-800 rounded-lg hover:bg-gray-300 transition-all text-sm font-medium"
                >
                  <Copy className="w-4 h-4" />
                  {copied ? 'Copied!' : 'Copy Link'}
                </button>
              </div>
            </div>

            {/* Step 2 */}
            <div className="space-y-3">
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 bg-blue-100 text-blue-600 rounded-full flex items-center justify-center font-bold text-sm">
                  2
                </div>
                <h3 className="text-lg font-semibold text-gray-900">Send Commands to @BotFather</h3>
              </div>
              <p className="text-sm text-gray-600 ml-11 mb-3">
                In the @BotFather chat, send these commands in order:
              </p>
              <div className="ml-11 space-y-2 text-sm">
                <code className="block bg-gray-100 p-3 rounded text-gray-900 font-mono">/newbot</code>
                <p className="text-gray-600">→ Choose a display name (e.g., "Operarius Bot")</p>
                <p className="text-gray-600 ml-4">→ Choose a unique username ending in "bot" (e.g., "operarius_bot")</p>
                <p className="text-gray-600 ml-4 font-semibold text-blue-600">
                  → Copy your bot token when @BotFather shows it
                </p>
              </div>
            </div>

            {/* Step 3 */}
            <div className="space-y-3">
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 bg-blue-100 text-blue-600 rounded-full flex items-center justify-center font-bold text-sm">
                  3
                </div>
                <h3 className="text-lg font-semibold text-gray-900">Paste Token in Operarius</h3>
              </div>
              <p className="text-sm text-gray-600 ml-11">
                Click "Next" below and paste your bot token. It looks like:
              </p>
              <code className="ml-11 block bg-gray-100 p-3 rounded text-gray-900 font-mono text-xs">
                123456789:ABCdefGHIjklMNOpqrSTUvwxYZ
              </code>
            </div>

            {/* Security Warning */}
            <div className="ml-11 bg-amber-50 border-l-4 border-amber-400 p-4 rounded">
              <div className="flex gap-3">
                <AlertCircle className="w-5 h-5 text-amber-600 flex-shrink-0 mt-0.5" />
                <div className="text-sm">
                  <p className="font-semibold text-amber-900">Keep your token secret!</p>
                  <p className="text-amber-800 mt-1">Never share your bot token publicly. Anyone with this token can control your bot.</p>
                </div>
              </div>
            </div>

            {/* Next Button */}
            <div className="ml-11 flex gap-3 mt-8">
              <button
                onClick={() => setStep('input')}
                className="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-all font-medium"
              >
                I've Got My Token →
              </button>
              <button
                onClick={onClose}
                className="px-6 py-2 bg-gray-200 text-gray-800 rounded-lg hover:bg-gray-300 transition-all font-medium"
              >
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
        <div className="bg-white rounded-2xl shadow-2xl max-w-md w-full">
          {/* Header */}
          <div className="bg-gradient-to-r from-blue-600 to-blue-700 text-white p-6 flex justify-between items-center">
            <h2 className="text-xl font-bold">Paste Bot Token</h2>
            <button
              onClick={onClose}
              className="p-2 hover:bg-white/20 rounded-lg transition-all"
            >
              <X className="w-5 h-5" />
            </button>
          </div>

          {/* Content */}
          <div className="p-8 space-y-4">
            <div>
              <label className="block text-sm font-semibold text-gray-900 mb-2">
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
                className="w-full px-4 py-3 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent font-mono text-sm"
              />
              <p className="text-xs text-gray-500 mt-2">Token is hidden for security. Paste from your clipboard.</p>
            </div>

            {error && (
              <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm">
                {error}
              </div>
            )}

            <div className="flex gap-3 pt-4">
              <button
                onClick={handleSetupTelegram}
                disabled={loading || !token.trim()}
                className="flex-1 px-4 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed transition-all font-medium"
              >
                {loading ? 'Configuring...' : 'Configure Telegram'}
              </button>
              <button
                onClick={() => setStep('guide')}
                disabled={loading}
                className="px-4 py-3 bg-gray-200 text-gray-800 rounded-lg hover:bg-gray-300 disabled:opacity-50 transition-all font-medium"
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
