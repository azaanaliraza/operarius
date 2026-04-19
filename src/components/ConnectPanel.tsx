import React from 'react';
import { TelegramSetupPanel } from './TelegramSetupPanel';

const ConnectPanel: React.FC<{ onClose: () => void }> = ({ onClose }) => {
  return <TelegramSetupPanel onClose={onClose} />;
};

export default ConnectPanel;
