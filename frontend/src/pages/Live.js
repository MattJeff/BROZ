import React, { useState, useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { useAuth } from '@/contexts/AuthContext';
import { Button } from '@/components/ui/button';
import { Heart, UserPlus, Flag, MessageCircle, ChevronRight } from 'lucide-react';
import BottomNav from '@/components/BottomNav';
import io from 'socket.io-client';
import { getAuthToken } from '@/utils/auth';
import { toast } from 'sonner';

const SOCKET_URL = process.env.REACT_APP_SOCKET_URL;
const API_URL = process.env.REACT_APP_BACKEND_URL;

const ICE_CONFIG = {
  iceServers: [
    { urls: 'stun:stun.l.google.com:19302' },
    { urls: 'stun:stun1.l.google.com:19302' },
  ],
  iceCandidatePoolSize: 10,
  bundlePolicy: 'max-bundle',
  rtcpMuxPolicy: 'require',
};

export const Live = () => {
  const { t } = useTranslation();
  const { user } = useAuth();
  const [searching, setSearching] = useState(false);
  const [connected, setConnected] = useState(false);
  const [partner, setPartner] = useState(null);
  const [chatMessage, setChatMessage] = useState('');
  const [messages, setMessages] = useState([]);
  const [showChat, setShowChat] = useState(false);

  const localVideoRef = useRef();
  const remoteVideoRef = useRef();
  const peerConnectionRef = useRef(null);
  const socketRef = useRef();
  const localStreamRef = useRef();
  const currentMatchIdRef = useRef(null);
  const pendingIceCandidatesRef = useRef([]);

  // Cleanup helper
  const cleanupPeerConnection = () => {
    if (peerConnectionRef.current) {
      peerConnectionRef.current.ontrack = null;
      peerConnectionRef.current.onicecandidate = null;
      peerConnectionRef.current.onconnectionstatechange = null;
      peerConnectionRef.current.close();
      peerConnectionRef.current = null;
    }
    if (remoteVideoRef.current) {
      remoteVideoRef.current.srcObject = null;
    }
    pendingIceCandidatesRef.current = [];
  };

  // Build join-queue payload from user profile
  const buildQueuePayload = () => ({
    display_name: user?.display_name || user?.username || 'Anonymous',
    bio: user?.bio || null,
    age: user?.age || 18,
    country: user?.country || null,
    kinks: user?.kinks || [],
    profile_photo_url: user?.profile_photo_url || null,
    filters: {
      country: null,
      age_min: null,
      age_max: null,
      kinks: [],
    },
  });

  useEffect(() => {
    const token = getAuthToken();
    if (!token) return;

    const socket = io(SOCKET_URL, {
      auth: { token },
      query: { token },
      transports: ['websocket', 'polling'],
      autoConnect: true,
    });
    socketRef.current = socket;

    socket.on('connected', () => {});

    // match-found: backend sends { match_id, partner: PartnerInfo }
    socket.on('match-found', async (data) => {
      const matchId = data.match_id;
      currentMatchIdRef.current = matchId;

      setPartner(data.partner);
      setSearching(false);

      // Get or reuse local stream
      let stream = localStreamRef.current;
      if (!stream || stream.getTracks().some(t => t.readyState !== 'live')) {
        stream = await startCamera();
        if (!stream) return;
      }

      // Cleanup previous PC
      cleanupPeerConnection();

      // Create new RTCPeerConnection
      const pc = new RTCPeerConnection(ICE_CONFIG);
      peerConnectionRef.current = pc;

      // Add local tracks
      stream.getTracks().forEach(track => {
        pc.addTrack(track, stream);
      });

      // Handle remote tracks
      pc.ontrack = (event) => {
        if (currentMatchIdRef.current !== matchId) return;
        if (remoteVideoRef.current && event.streams[0]) {
          remoteVideoRef.current.srcObject = event.streams[0];
          remoteVideoRef.current.play().catch(() => {});
          setConnected(true);
        }
      };

      // Handle ICE candidates -> relay via webrtc-signal
      pc.onicecandidate = (event) => {
        if (event.candidate && currentMatchIdRef.current === matchId) {
          socket.emit('webrtc-signal', {
            type: 'ice-candidate',
            candidate: event.candidate,
            match_id: matchId,
          });
        }
      };

      pc.onconnectionstatechange = () => {
        if (pc.connectionState === 'connected') {
          setConnected(true);
        } else if (pc.connectionState === 'failed') {
          // Connection failed
        }
      };

      // If initiator, create and send offer
      if (data.is_initiator) {
        await new Promise(r => setTimeout(r, 300));
        if (currentMatchIdRef.current !== matchId) return;
        try {
          const offer = await pc.createOffer();
          await pc.setLocalDescription(offer);
          socket.emit('webrtc-signal', {
            type: 'offer',
            sdp: offer.sdp,
            match_id: matchId,
          });
        } catch (err) {
          console.error('[Live] Offer error:', err);
        }
      }
    });

    // Handle webrtc-signal (offer/answer/ice-candidate multiplexed)
    socket.on('webrtc-signal', async (data) => {
      const pc = peerConnectionRef.current;
      if (!pc) return;
      if (data.match_id && data.match_id !== currentMatchIdRef.current) return;
      if (pc.signalingState === 'closed') return;

      try {
        if (data.type === 'offer') {
          if (pc.signalingState !== 'stable') return;
          pendingIceCandidatesRef.current = [];
          await pc.setRemoteDescription(new RTCSessionDescription({ type: 'offer', sdp: data.sdp }));
          // Apply buffered candidates
          for (const c of pendingIceCandidatesRef.current) {
            await pc.addIceCandidate(new RTCIceCandidate(c)).catch(() => {});
          }
          pendingIceCandidatesRef.current = [];
          const answer = await pc.createAnswer();
          await pc.setLocalDescription(answer);
          socket.emit('webrtc-signal', {
            type: 'answer',
            sdp: answer.sdp,
            match_id: currentMatchIdRef.current,
          });
        } else if (data.type === 'answer') {
          if (pc.signalingState !== 'have-local-offer') return;
          await pc.setRemoteDescription(new RTCSessionDescription({ type: 'answer', sdp: data.sdp }));
          for (const c of pendingIceCandidatesRef.current) {
            await pc.addIceCandidate(new RTCIceCandidate(c)).catch(() => {});
          }
          pendingIceCandidatesRef.current = [];
        } else if (data.type === 'ice-candidate' && data.candidate) {
          if (pc.remoteDescription) {
            await pc.addIceCandidate(new RTCIceCandidate(data.candidate));
          } else {
            pendingIceCandidatesRef.current.push(data.candidate);
          }
        }
      } catch (err) {
        console.error('[Live] Signal error:', err);
      }
    });

    // Partner left / disconnected
    socket.on('partner-left', () => {
      currentMatchIdRef.current = null;
      cleanupPeerConnection();
      setConnected(false);
      setPartner(null);
      setMessages([]);
      toast.info('Partner disconnected');
    });

    socket.on('partner-disconnected', () => {
      currentMatchIdRef.current = null;
      cleanupPeerConnection();
      setConnected(false);
      setPartner(null);
      setMessages([]);
      toast.info('Partner disconnected');
    });

    // Chat messages from partner
    socket.on('chat-message', (data) => {
      setMessages((prev) => [...prev, { from: 'partner', text: data.content }]);
    });

    // Like received
    socket.on('like-received', () => {
      toast.success('You received a like!');
    });

    socket.on('error', (data) => {
      console.error('[Live] Socket error:', data);
    });

    return () => {
      socket.disconnect();
      cleanupPeerConnection();
      if (localStreamRef.current) {
        localStreamRef.current.getTracks().forEach((track) => track.stop());
      }
    };
  }, []);

  const startCamera = async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({
        video: true,
        audio: true,
      });
      localStreamRef.current = stream;
      if (localVideoRef.current) {
        localVideoRef.current.srcObject = stream;
      }
      return stream;
    } catch (error) {
      console.error('Camera access error:', error);
      toast.error('Unable to access camera/microphone');
      return null;
    }
  };

  const handleNext = () => {
    // End current call
    if (currentMatchIdRef.current && socketRef.current?.connected) {
      socketRef.current.emit('end-call', { match_id: currentMatchIdRef.current });
    }
    currentMatchIdRef.current = null;
    cleanupPeerConnection();
    setConnected(false);
    setPartner(null);
    setMessages([]);

    // Rejoin queue
    if (socketRef.current?.connected) {
      socketRef.current.emit('next-match', buildQueuePayload());
      setSearching(true);
    }
  };

  const handleGoLive = async () => {
    setSearching(true);
    const stream = await startCamera();

    if (!stream) {
      setSearching(false);
      return;
    }

    if (socketRef.current?.connected) {
      socketRef.current.emit('join-queue', buildQueuePayload());
    }
  };

  const handleLike = () => {
    if (!partner || !currentMatchIdRef.current || !socketRef.current?.connected) return;
    socketRef.current.emit('send-like', { match_id: currentMatchIdRef.current });
  };

  const handleFollow = () => {
    if (!partner || !socketRef.current?.connected) return;
    socketRef.current.emit('send-follow-request', { target_id: partner.user_id });
  };

  const handleReport = async () => {
    if (!partner) return;
    try {
      const token = getAuthToken();
      await fetch(`${API_URL}/api/moderation/reports`, {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${token}`,
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ reported_user_id: partner.user_id, reason: 'inappropriate' }),
      });
      toast.success('Reported successfully');
      handleNext();
    } catch (error) {
      console.error('Report error:', error);
    }
  };

  const handleSendMessage = () => {
    if (!chatMessage.trim() || !connected || !currentMatchIdRef.current) return;

    socketRef.current.emit('chat-message', {
      match_id: currentMatchIdRef.current,
      content: chatMessage,
    });
    setMessages((prev) => [...prev, { from: 'me', text: chatMessage }]);
    setChatMessage('');
  };

  return (
    <div className="min-h-screen bg-[#050505] text-white pb-16">
      <div className="relative w-full h-[calc(100vh-64px)]" data-testid="live-video-container">
        {connected && (
          <video
            ref={remoteVideoRef}
            autoPlay
            playsInline
            className="absolute inset-0 w-full h-full object-cover"
            data-testid="remote-video"
          />
        )}

        <video
          ref={localVideoRef}
          autoPlay
          playsInline
          muted
          className="absolute top-4 right-4 w-32 h-48 object-cover rounded-lg border border-white/20 z-10"
          data-testid="local-video"
        />

        {searching && (
          <div className="absolute inset-0 flex items-center justify-center bg-black/90 backdrop-blur-sm z-20" data-testid="searching-overlay">
            <div className="text-center">
              <div className="animate-spin rounded-full h-16 w-16 border-t-2 border-b-2 border-white mx-auto mb-4"></div>
              <p className="text-xl font-bold">{t('live.searching')}</p>
            </div>
          </div>
        )}

        {!searching && !connected && (
          <div className="absolute inset-0 flex flex-col items-center justify-center gap-6" data-testid="prematch-screen">
            <Button
              data-testid="go-live-btn"
              onClick={handleGoLive}
              className="h-16 px-12 text-xl rounded-lg bg-white text-black hover:bg-white/90 font-black shadow-[0_0_40px_rgba(255,255,255,0.1)] transition-all duration-300 active:scale-95"
              style={{ fontFamily: 'Unbounded, sans-serif' }}
            >
              {t('live.go_live')}
            </Button>
          </div>
        )}

        {connected && partner && (
          <>
            <div className="absolute bottom-20 left-4 z-30 bg-black/80 backdrop-blur-xl border border-white/10 rounded-lg p-4" data-testid="partner-info">
              <h3 className="text-lg font-bold">{partner.display_name}</h3>
              <p className="text-sm text-white/60">{partner.age} years old</p>
              {partner.kinks && partner.kinks.length > 0 && (
                <div className="flex flex-wrap gap-1 mt-2">
                  {partner.kinks.slice(0, 3).map((kink) => (
                    <span key={kink} className="text-xs bg-white/10 px-2 py-1 rounded">
                      {kink}
                    </span>
                  ))}
                </div>
              )}
            </div>

            <div className="absolute right-4 bottom-32 z-30 flex flex-col gap-3" data-testid="action-buttons">
              <Button
                data-testid="like-btn"
                onClick={handleLike}
                size="icon"
                className="h-12 w-12 rounded-lg bg-black/80 backdrop-blur-xl border border-white/10 hover:bg-white/10"
              >
                <Heart className="w-5 h-5" />
              </Button>

              <Button
                data-testid="follow-btn"
                onClick={handleFollow}
                size="icon"
                className="h-12 w-12 rounded-lg bg-black/80 backdrop-blur-xl border border-white/10 hover:bg-white/10"
              >
                <UserPlus className="w-5 h-5" />
              </Button>

              <Button
                data-testid="chat-btn"
                onClick={() => setShowChat(!showChat)}
                size="icon"
                className="h-12 w-12 rounded-lg bg-black/80 backdrop-blur-xl border border-white/10 hover:bg-white/10"
              >
                <MessageCircle className="w-5 h-5" />
              </Button>
            </div>

            <div className="absolute top-4 left-4 z-30 flex gap-2">
              <Button
                data-testid="report-btn"
                onClick={handleReport}
                size="icon"
                className="h-10 w-10 rounded-lg bg-black/80 backdrop-blur-xl border border-white/10 hover:bg-red-500/20 hover:border-red-500/50"
              >
                <Flag className="w-5 h-5" />
              </Button>
            </div>

            <div className="absolute bottom-20 left-1/2 transform -translate-x-1/2 z-30">
              <Button
                data-testid="next-btn"
                onClick={handleNext}
                className="h-12 px-8 rounded-lg bg-white text-black hover:bg-white/90 font-bold flex items-center gap-2"
              >
                {t('live.next')}
                <ChevronRight className="w-4 h-4" />
              </Button>
            </div>

            {showChat && (
              <div className="absolute bottom-0 left-0 right-0 h-1/2 bg-black/90 backdrop-blur-xl border-t border-white/10 z-40 flex flex-col" data-testid="chat-overlay">
                <div className="flex-1 overflow-y-auto p-4 space-y-2">
                  {messages.map((msg, idx) => (
                    <div
                      key={idx}
                      className={`p-3 rounded-lg max-w-[70%] ${
                        msg.from === 'me'
                          ? 'bg-white text-black ml-auto'
                          : 'bg-white/10'
                      }`}
                    >
                      <p className="text-sm">{msg.text}</p>
                    </div>
                  ))}
                </div>
                <div className="p-4 border-t border-white/10 flex gap-2">
                  <input
                    type="text"
                    data-testid="chat-input"
                    value={chatMessage}
                    onChange={(e) => setChatMessage(e.target.value)}
                    onKeyPress={(e) => e.key === 'Enter' && handleSendMessage()}
                    placeholder="Type a message..."
                    className="flex-1 bg-white/10 border border-white/10 rounded-lg px-4 py-2 text-white placeholder:text-white/40 focus:outline-none focus:border-white/20"
                  />
                  <Button
                    data-testid="send-message-btn"
                    onClick={handleSendMessage}
                    className="rounded-lg px-6 bg-white text-black hover:bg-white/90"
                  >
                    Send
                  </Button>
                </div>
              </div>
            )}
          </>
        )}
      </div>

      <BottomNav />
    </div>
  );
};

export default Live;
