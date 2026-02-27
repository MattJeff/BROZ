import React, { useState, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';

const API_URL = process.env.REACT_APP_BACKEND_URL;

export const PlayShow = () => {
  const navigate = useNavigate();
  const [messageUnreadCount, setMessageUnreadCount] = useState(0);

  // Fetch unread messages count
  const fetchMessageUnreadCount = useCallback(async () => {
    try {
      const token = localStorage.getItem('brozr_token');
      if (!token) return;
      
      const res = await fetch(`${API_URL}/api/messages/unread-count`, {
        headers: { 'Authorization': `Bearer ${token}` }
      });
      if (res.ok) {
        const data = await res.json();
        setMessageUnreadCount(data.count);
      }
    } catch (err) {
      console.error('Failed to fetch message unread count:', err);
    }
  }, []);

  useEffect(() => {
    fetchMessageUnreadCount();
    const interval = setInterval(fetchMessageUnreadCount, 30000);
    return () => clearInterval(interval);
  }, [fetchMessageUnreadCount]);

  return (
    <div className="fixed inset-0 bg-black flex items-center justify-center">
      {/* Mobile-width container for desktop */}
      <div className="relative w-full h-full max-w-[430px] mx-auto bg-black flex flex-col overflow-hidden">
        {/* Header */}
        <div className="flex-shrink-0 p-4 flex items-center justify-between">
          <button onClick={() => navigate('/live-prematch')} className="text-white/70 hover:text-white">
            <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
            </svg>
          </button>
          <h1 className="text-white font-bold text-lg">Play Show</h1>
          <div className="w-6"></div>
        </div>

        {/* Content */}
        <div className="flex-1 flex flex-col items-center justify-center px-8 text-center">
          {/* Icon */}
          <div className="w-24 h-24 rounded-full bg-white/5 flex items-center justify-center mb-8">
            <svg className="w-12 h-12 text-white/50" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <rect x="3" y="3" width="18" height="18" rx="3" strokeWidth={1.5} />
              <path d="M10 8l6 4-6 4V8z" strokeWidth={1.5} strokeLinejoin="round" />
            </svg>
          </div>

          {/* Title */}
          <h2 className="text-white text-2xl font-bold mb-4">Très bientôt</h2>
          
          {/* Description */}
          <p className="text-white text-lg font-medium leading-relaxed max-w-sm mb-4">
            Live Streaming de tes Bros préférés.
          </p>
          <p className="text-white/50 text-base leading-relaxed max-w-sm">
            Regarde, interagis et envoie des tips à des utilisateurs en direct.
          </p>

          {/* Badge */}
          <div className="mt-8 px-4 py-2 bg-white/5 rounded-full border border-white/10">
            <span className="text-white/50 text-sm font-medium">À venir</span>
          </div>
        </div>

        {/* Bottom Nav - Same structure as LivePrematch */}
        <nav className="flex-shrink-0 bg-black px-4 py-3 pb-6">
          <div className="flex justify-around max-w-md mx-auto">
            <button onClick={() => navigate('/live-prematch')} className="flex flex-col items-center gap-1 text-white/50">
              <svg className="w-6 h-6" fill="currentColor" viewBox="0 0 24 24"><path d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" /></svg>
              <span className="text-xs font-medium">Cam Live</span>
            </button>
            <button onClick={() => navigate('/space')} className="relative flex flex-col items-center gap-1 text-white/50">
              <svg className="w-6 h-6" fill="currentColor" viewBox="0 0 24 24">
                <path d="M21 6h-2v9H6v2c0 .55.45 1 1 1h11l4 4V7c0-.55-.45-1-1-1zm-4 6V3c0-.55-.45-1-1-1H3c-.55 0-1 .45-1 1v14l4-4h10c.55 0 1-.45 1-1z"/>
              </svg>
              <span className="text-xs font-medium">Space</span>
              {/* Unread message badge */}
              {messageUnreadCount > 0 && (
                <div className="absolute -top-1 right-1 min-w-[18px] h-[18px] bg-red-500 rounded-full flex items-center justify-center">
                  <span className="text-white text-[10px] font-bold">{messageUnreadCount > 9 ? '9+' : messageUnreadCount}</span>
                </div>
              )}
            </button>
            <button className="flex flex-col items-center gap-1 text-white">
              <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <rect x="3" y="3" width="18" height="18" rx="3" strokeWidth={1.5} />
                <path d="M10 8l6 4-6 4V8z" strokeWidth={1.5} strokeLinejoin="round" />
              </svg>
              <span className="text-xs font-medium">Play Show</span>
            </button>
          </div>
        </nav>
      </div>
    </div>
  );
};

export default PlayShow;
