import React, { useEffect, useState, useRef } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { useAuth } from '@/contexts/AuthContext';
import io from 'socket.io-client';

const GlobalLiveCamListener = () => {
  const navigate = useNavigate();
  const location = useLocation();
  const { user } = useAuth();
  const socketRef = useRef(null);
  const [incomingRequest, setIncomingRequest] = useState(null);
  const [dismissed, setDismissed] = useState(false);
  const touchStartY = useRef(null);
  const bannerRef = useRef(null);

  // Check if a user is currently ignored (60 min TTL)
  const isUserIgnored = (userId) => {
    try {
      const ignored = JSON.parse(localStorage.getItem('brozr_ignored_users') || '{}');
      const timestamp = ignored[userId];
      if (!timestamp) return false;
      const elapsed = Date.now() - timestamp;
      if (elapsed > 60 * 60 * 1000) {
        // Expired, clean up
        delete ignored[userId];
        localStorage.setItem('brozr_ignored_users', JSON.stringify(ignored));
        return false;
      }
      return true;
    } catch {
      return false;
    }
  };

  const handleIgnore = () => {
    if (!incomingRequest) return;
    // Store caller in ignored list with timestamp
    try {
      const ignored = JSON.parse(localStorage.getItem('brozr_ignored_users') || '{}');
      ignored[incomingRequest.caller_id] = Date.now();
      localStorage.setItem('brozr_ignored_users', JSON.stringify(ignored));
    } catch { /* ignore */ }
    // Decline the call silently
    if (socketRef.current && incomingRequest.call_id) {
      socketRef.current.emit('call-decline', { call_id: incomingRequest.call_id });
    }
    setIncomingRequest(null);
  };

  useEffect(() => {
    const token = localStorage.getItem('brozr_token');
    if (!token || !user) return;

    // Don't connect if already on Space (Space.js handles its own socket + call UI)
    if (location.pathname === '/space') {
      if (socketRef.current) {
        socketRef.current.disconnect();
        socketRef.current = null;
      }
      setIncomingRequest(null);
      return;
    }

    // Only create socket if not already connected
    if (socketRef.current?.connected) return;

    const MESSAGING_SOCKET_URL = process.env.REACT_APP_MESSAGING_SOCKET_URL;
    const socket = io(MESSAGING_SOCKET_URL, {
      auth: { token },
      query: { token },
      transports: ['websocket', 'polling'],
      reconnection: true,
      reconnectionAttempts: 3,
      reconnectionDelay: 2000
    });
    socketRef.current = socket;

    socket.on('incoming-call', (data) => {
      if (isUserIgnored(data.caller_id)) {
        socket.emit('call-decline', { call_id: data.call_id });
        return;
      }
      setDismissed(false);
      setIncomingRequest({
        call_id: data.call_id,
        room_id: data.room_id,
        sfu_token: data.sfu_token,
        caller_id: data.caller_id,
        caller_name: data.caller_name,
        caller_photo: data.caller_photo
      });
    });

    socket.on('call-declined', () => {
      setIncomingRequest(null);
    });

    socket.on('call-ended', () => {
      setIncomingRequest(null);
    });

    return () => {
      socket.disconnect();
      socketRef.current = null;
    };
  }, [user, location.pathname]);

  const handleAccept = () => {
    if (!incomingRequest) return;
    const callData = { ...incomingRequest };
    setIncomingRequest(null);
    navigate('/space', { state: { incomingCall: callData } });
  };

  const handleDecline = () => {
    if (!incomingRequest) return;
    if (socketRef.current && incomingRequest.call_id) {
      socketRef.current.emit('call-decline', { call_id: incomingRequest.call_id });
    }
    setIncomingRequest(null);
  };

  // Swipe up to dismiss (decline)
  const onTouchStart = (e) => {
    touchStartY.current = e.touches[0].clientY;
  };
  const onTouchMove = (e) => {
    if (touchStartY.current === null) return;
    const delta = e.touches[0].clientY - touchStartY.current;
    if (bannerRef.current) {
      // Only allow upward swipe
      bannerRef.current.style.transform = `translateY(${Math.min(0, delta)}px)`;
      bannerRef.current.style.opacity = Math.max(0, 1 + delta / 150);
    }
  };
  const onTouchEnd = (e) => {
    if (touchStartY.current === null) return;
    const delta = e.changedTouches[0].clientY - touchStartY.current;
    touchStartY.current = null;
    if (delta < -60) {
      // Swiped up enough → decline
      setDismissed(true);
      setTimeout(() => handleDecline(), 200);
    } else if (bannerRef.current) {
      // Reset position
      bannerRef.current.style.transform = '';
      bannerRef.current.style.opacity = '';
    }
  };

  if (!incomingRequest || dismissed || location.pathname === '/space') {
    return null;
  }

  return (
    <>
      <div
        ref={bannerRef}
        className="fixed top-0 inset-x-0 z-[9999] p-3 pt-[env(safe-area-inset-top,12px)] call-banner-slide-down"
        onTouchStart={onTouchStart}
        onTouchMove={onTouchMove}
        onTouchEnd={onTouchEnd}
        data-testid="incoming-call-banner"
      >
        <div className="mx-auto max-w-md bg-[#1c1c1e]/95 backdrop-blur-xl rounded-2xl border border-white/10 shadow-2xl overflow-hidden">
          {/* Swipe indicator */}
          <div className="flex justify-center pt-2 pb-1">
            <div className="w-8 h-1 rounded-full bg-white/20" />
          </div>

          {/* Banner content */}
          <div className="px-4 pb-3">
            {/* Top row: photo + info */}
            <div className="flex items-center gap-3 mb-3">
              {/* Caller photo */}
              <div className="w-10 h-10 rounded-full overflow-hidden flex-shrink-0 border-2 border-green-500/50">
                {incomingRequest.caller_photo ? (
                  <img src={incomingRequest.caller_photo} alt="" className="w-full h-full object-cover" />
                ) : (
                  <div className="w-full h-full bg-white/20 flex items-center justify-center text-white font-bold text-sm">
                    {incomingRequest.caller_name?.[0]?.toUpperCase() || '?'}
                  </div>
                )}
              </div>

              {/* Caller info */}
              <div className="flex-1 min-w-0">
                <p className="text-white font-semibold text-sm truncate">@{incomingRequest.caller_name}</p>
                <p className="text-white/50 text-xs">Appel vidéo entrant</p>
              </div>

              {/* Camera icon */}
              <div className="w-8 h-8 rounded-full bg-green-500/20 flex items-center justify-center flex-shrink-0">
                <svg className="w-4 h-4 text-green-400" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
                </svg>
              </div>
            </div>

            {/* Action buttons */}
            <div className="flex gap-2">
              <button
                onClick={handleAccept}
                className="flex-1 py-2.5 bg-green-500 hover:bg-green-600 text-white text-sm font-semibold rounded-xl transition-colors"
              >
                Accepter
              </button>
              <button
                onClick={handleDecline}
                className="flex-1 py-2.5 bg-red-500 hover:bg-red-600 text-white text-sm font-semibold rounded-xl transition-colors"
              >
                Refuser
              </button>
              <button
                onClick={handleIgnore}
                className="py-2.5 px-3 bg-white/10 hover:bg-white/15 text-white/70 text-sm font-semibold rounded-xl transition-colors flex flex-col items-center justify-center"
              >
                <span className="text-xs font-bold">Ignorer</span>
                <span className="text-[9px] text-white/40">60 minutes</span>
              </button>
            </div>
          </div>
        </div>
      </div>

      <style>{`
        @keyframes slide-down-banner {
          from { transform: translateY(-100%); opacity: 0; }
          to { transform: translateY(0); opacity: 1; }
        }
        .call-banner-slide-down {
          animation: slide-down-banner 0.35s cubic-bezier(0.16, 1, 0.3, 1);
        }
      `}</style>
    </>
  );
};

export default GlobalLiveCamListener;
