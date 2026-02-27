import React, { useState, useEffect, useRef, useCallback } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { useAuth } from '@/contexts/AuthContext';
import { toast } from 'sonner';
import { io } from 'socket.io-client';
import { KINKS_FLAT } from '@/utils/kinks';

const API_URL = process.env.REACT_APP_BACKEND_URL;

const STUN_SERVERS = [
  { urls: 'stun:stun.l.google.com:19302' },
  { urls: 'stun:stun1.l.google.com:19302' },
  { urls: 'stun:stun2.l.google.com:19302' },
  { urls: 'stun:stun3.l.google.com:19302' },
  { urls: 'stun:stun4.l.google.com:19302' },
  // Additional STUN servers for redundancy
  { urls: 'stun:stun.services.mozilla.com:3478' },
  { urls: 'stun:stun.stunprotocol.org:3478' },
  // OpenRelay TURN servers (free, public)
  {
    urls: 'turn:openrelay.metered.ca:80',
    username: 'openrelayproject',
    credential: 'openrelayproject'
  },
  {
    urls: 'turn:openrelay.metered.ca:443',
    username: 'openrelayproject',
    credential: 'openrelayproject'
  },
  {
    urls: 'turn:openrelay.metered.ca:443?transport=tcp',
    username: 'openrelayproject',
    credential: 'openrelayproject'
  }
];

// ICE restart configuration for better reconnection
const ICE_CONFIG = {
  iceServers: STUN_SERVERS,
  iceCandidatePoolSize: 10,
  bundlePolicy: 'max-bundle',
  rtcpMuxPolicy: 'require'
};

// Country data with flags
const countryList = [
  { code: 'FR', name: 'France', flag: '🇫🇷' },
  { code: 'BE', name: 'Belgique', flag: '🇧🇪' },
  { code: 'CH', name: 'Suisse', flag: '🇨🇭' },
  { code: 'CA', name: 'Canada', flag: '🇨🇦' },
  { code: 'US', name: 'États-Unis', flag: '🇺🇸' },
  { code: 'UK', name: 'Royaume-Uni', flag: '🇬🇧' },
  { code: 'DE', name: 'Allemagne', flag: '🇩🇪' },
  { code: 'ES', name: 'Espagne', flag: '🇪🇸' },
  { code: 'IT', name: 'Italie', flag: '🇮🇹' },
  { code: 'NL', name: 'Pays-Bas', flag: '🇳🇱' },
  { code: 'PT', name: 'Portugal', flag: '🇵🇹' },
  { code: 'AT', name: 'Autriche', flag: '🇦🇹' }
];

// Country flags mapping for quick lookup
const countryFlags = {
  FR: '🇫🇷', US: '🇺🇸', UK: '🇬🇧', DE: '🇩🇪', ES: '🇪🇸', IT: '🇮🇹', 
  PT: '🇵🇹', NL: '🇳🇱', BE: '🇧🇪', CH: '🇨🇭', CA: '🇨🇦', AU: '🇦🇺',
  BR: '🇧🇷', MX: '🇲🇽', AR: '🇦🇷', JP: '🇯🇵', KR: '🇰🇷', GB: '🇬🇧', ALL: '🌍'
};

// Kink categories (using centralized data)
const kinkCategories = KINKS_FLAT;

// KinkChip component - White style like LivePrematch
function KinkChip({ label, selected, onToggle }) {
  const cls = selected
    ? "px-3 py-1.5 rounded-full text-sm bg-white text-black transition-all"
    : "px-3 py-1.5 rounded-full text-sm bg-white/5 text-white/70 hover:bg-white/10 border border-white/10 transition-all";
  return <button type="button" onClick={onToggle} className={cls}>{label}</button>;
}

// DualRangeSlider component - White style like LivePrematch
function DualRangeSlider({ min, max, minVal, maxVal, onMinChange, onMaxChange }) {
  const range = max - min;
  const minPercent = ((minVal - min) / range) * 100;
  const maxPercent = ((maxVal - min) / range) * 100;

  return (
    <div className="relative h-6 flex items-center">
      <div className="absolute w-full h-2 bg-white/10 rounded-full"></div>
      <div 
        className="absolute h-2 bg-white rounded-full"
        style={{ left: minPercent + '%', right: (100 - maxPercent) + '%' }}
      ></div>
      <input
        type="range"
        min={min}
        max={max}
        value={minVal}
        onChange={(e) => {
          const val = Math.min(Number(e.target.value), maxVal - 1);
          onMinChange(val);
        }}
        className="absolute w-full h-2 appearance-none bg-transparent pointer-events-none [&::-webkit-slider-thumb]:pointer-events-auto [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-5 [&::-webkit-slider-thumb]:h-5 [&::-webkit-slider-thumb]:bg-white [&::-webkit-slider-thumb]:border-2 [&::-webkit-slider-thumb]:border-white [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:cursor-pointer [&::-webkit-slider-thumb]:shadow-lg [&::-moz-range-thumb]:pointer-events-auto [&::-moz-range-thumb]:appearance-none [&::-moz-range-thumb]:w-5 [&::-moz-range-thumb]:h-5 [&::-moz-range-thumb]:bg-white [&::-moz-range-thumb]:border-2 [&::-moz-range-thumb]:border-white [&::-moz-range-thumb]:rounded-full [&::-moz-range-thumb]:cursor-pointer"
        style={{ zIndex: minVal > max - 10 ? 5 : 3 }}
      />
      <input
        type="range"
        min={min}
        max={max}
        value={maxVal}
        onChange={(e) => {
          const val = Math.max(Number(e.target.value), minVal + 1);
          onMaxChange(val);
        }}
        className="absolute w-full h-2 appearance-none bg-transparent pointer-events-none [&::-webkit-slider-thumb]:pointer-events-auto [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-5 [&::-webkit-slider-thumb]:h-5 [&::-webkit-slider-thumb]:bg-white [&::-webkit-slider-thumb]:border-2 [&::-webkit-slider-thumb]:border-white [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:cursor-pointer [&::-webkit-slider-thumb]:shadow-lg [&::-moz-range-thumb]:pointer-events-auto [&::-moz-range-thumb]:appearance-none [&::-moz-range-thumb]:w-5 [&::-moz-range-thumb]:h-5 [&::-moz-range-thumb]:bg-white [&::-moz-range-thumb]:border-2 [&::-moz-range-thumb]:border-white [&::-moz-range-thumb]:rounded-full [&::-moz-range-thumb]:cursor-pointer"
        style={{ zIndex: 4 }}
      />
    </div>
  );
}

// Pre-calculated random values for hearts to avoid Math.random during render
const heartPositions = [28, 45, 62, 35, 78, 52, 40, 68];
const heartDurations = [1.6, 1.8, 1.5, 1.9, 1.7, 1.55, 1.85, 1.65];

// Floating hearts animation component
const FloatingHearts = ({ show }) => {
  if (!show) return null;
  return (
    <div className="fixed inset-0 pointer-events-none z-50 overflow-hidden">
      {[...Array(8)].map((_, i) => (
        <div
          key={i}
          className="absolute text-4xl animate-float-up"
          style={{
            left: `${heartPositions[i]}%`,
            bottom: '20%',
            animationDelay: `${i * 0.1}s`,
            animationDuration: `${heartDurations[i]}s`
          }}
        >
          ❤️
        </div>
      ))}
      <style>{`
        @keyframes float-up {
          0% { transform: translateY(0) scale(0.8); opacity: 1; }
          50% { transform: translateY(-30vh) scale(1.1); opacity: 0.8; }
          100% { transform: translateY(-60vh) scale(0.6); opacity: 0; }
        }
        .animate-float-up { animation: float-up 1.8s ease-out forwards; }
      `}</style>
    </div>
  );
};

const VideoCall = () => {
  const navigate = useNavigate();
  const location = useLocation();
  const { user, token } = useAuth();
  
  // Refs
  const socketRef = useRef(null);
  const localVideoRef = useRef(null);
  const remoteVideoRef = useRef(null);
  const peerConnectionRef = useRef(null);
  const localStreamRef = useRef(null);
  const isSwitchingCameraRef = useRef(false); // Flag to prevent disconnect during camera switch
  const swipeStartX = useRef(0);
  const swipeStartY = useRef(0);
  const videoContainerRef = useRef(null);
  const currentMatchIdRef = useRef(null); // Track current match ID to filter stale signals
  const pendingIceCandidatesRef = useRef([]); // Buffer for ICE candidates received before remote description
  const clonedStreamRef = useRef(null);
  const healthCheckRef = useRef(null);

  // State
  const [connectionState, setConnectionState] = useState('initializing');
  const [partner, setPartner] = useState(null);
  const [queueSize, setQueueSize] = useState(0);
  const [error, setError] = useState(null);
  const [facingMode, setFacingMode] = useState('user');
  const [likesReceived, setLikesReceived] = useState(() => {
    // Initialize from user's total_likes if available
    return user?.total_likes || 0;
  });
  const [liked, setLiked] = useState(false);
  const [showHearts, setShowHearts] = useState(false);
  const [showFollowPopup, setShowFollowPopup] = useState(false);
  const [showIncomingFollowRequest, setShowIncomingFollowRequest] = useState(false);
  const [followRequester, setFollowRequester] = useState(null);
  const [followSent, setFollowSent] = useState(false);
  const [followStatus, setFollowStatus] = useState(null); // 'pending', 'accepted', 'refused'
  const [isMutualFollow, setIsMutualFollow] = useState(false); // Both users following each other
  const [showKinksOverlay, setShowKinksOverlay] = useState(false);
  const [showJuiceOverlay, setShowJuiceOverlay] = useState(false);
  const [showReportOverlay, setShowReportOverlay] = useState(false);
  const [showReportConfirm, setShowReportConfirm] = useState(false); // Report sent confirmation
  const [reportStep, setReportStep] = useState(1); // 1: choose reason, 2: optional comment
  const [reportReason, setReportReason] = useState('');
  const [reportComment, setReportComment] = useState('');
  const [showFiltersOverlay, setShowFiltersOverlay] = useState(false);
  const [showChatInput, setShowChatInput] = useState(false);
  const [chatMessage, setChatMessage] = useState('');
  const [chatMessages, setChatMessages] = useState([]);
  const [localStreamReady, setLocalStreamReady] = useState(false);
  const [swipeProgress, setSwipeProgress] = useState(0); // 0-100 for swipe animation
  const [showUnfollowConfirm, setShowUnfollowConfirm] = useState(false); // Confirm unfollow popup
  const followingIdsRef = useRef(new Set()); // Pre-loaded set of profile IDs we follow

  // Notification state (for bell icon)
  const [showNotifications, setShowNotifications] = useState(false);
  const [notifications, setNotifications] = useState([]);
  const [unreadCount, setUnreadCount] = useState(0);
  
  // Detect if device is touch-capable (mobile)
  const isMobile = typeof window !== 'undefined' && 
    (('ontouchstart' in window) || 
     (navigator.maxTouchPoints > 0) ||
     window.matchMedia('(pointer: coarse)').matches);
  
  // Filter state (same as LivePrematch)
  const [tempCountry, setTempCountry] = useState('ALL');
  const [tempAgeMin, setTempAgeMin] = useState(18);
  const [tempAgeMax, setTempAgeMax] = useState(60);
  const [tempDistance, setTempDistance] = useState(400);
  const [tempKinks, setTempKinks] = useState([]);
  const [activeKinks, setActiveKinks] = useState([]);
  
  // Get saved filters from localStorage - use state for reactivity
  const [savedFilters, setSavedFilters] = useState(() => {
    try {
      const saved = localStorage.getItem('brozr_match_filters');
      if (saved) return JSON.parse(saved);
    } catch (e) {}
    return null;
  });
  
  // Get filters from navigation state or sessionStorage (set by LivePrematch via SafetyScreen)
  const filters = React.useMemo(() => {
    // First try location state
    if (location.state?.filters) {
      return location.state.filters;
    }
    // Then try localStorage (persisted across sessions)
    const storedFilters = localStorage.getItem('brozr_match_filters');
    if (storedFilters) {
      try {
        return JSON.parse(storedFilters);
      } catch (e) {
        return {};
      }
    }
    return {};
  }, [location.state?.filters]);

  // Initialize temp filters from passed filters
  useEffect(() => {
    if (filters) {
      setTempCountry(filters.country || 'ALL');
      setTempAgeMin(filters.ageMin || 18);
      setTempAgeMax(filters.ageMax || 60);
      setTempDistance(filters.distance || 400);
      setTempKinks(filters.kinks || []);
      setActiveKinks(filters.kinks || []);
    }
  }, [filters]);

  // Calculate total active filters count (age, distance, kinks)
  const getActiveFiltersCount = () => {
    if (!savedFilters) return 0;
    let count = (savedFilters.kinks && savedFilters.kinks.length) || 0;
    // Count age if modified from defaults (18-60)
    const minAge = savedFilters.minAge || savedFilters.ageMin;
    const maxAge = savedFilters.maxAge || savedFilters.ageMax;
    if ((minAge && minAge !== 18) || (maxAge && maxAge !== 60)) {
      count++;
    }
    // Count distance if modified from default (400)
    if (savedFilters.distance && savedFilters.distance !== 400) {
      count++;
    }
    return count;
  };

  // Initialize likes from user's total_likes and fetch fresh from API
  useEffect(() => {
    if (user && user.total_likes !== undefined) {
      setLikesReceived(user.total_likes);
    }
    
    // Also fetch fresh likes count from API to ensure accuracy
    const fetchLikesCount = async () => {
      try {
        const token = localStorage.getItem('brozr_token');
        if (!token) return;
        const response = await fetch(`${process.env.REACT_APP_BACKEND_URL}/api/auth/me`, {
          headers: { 'Authorization': `Bearer ${token}` }
        });
        if (response.ok) {
          const userData = await response.json();
          if (userData.total_likes !== undefined) {
            setLikesReceived(userData.total_likes);
          }
        }
      } catch (err) {
        console.error('Error fetching likes count:', err);
      }
    };
    fetchLikesCount();
  }, [user]);

  // Get matching kinks - based on user's "Looking for" filters, fallback to user profile kinks
  const getMatchingKinks = useCallback(() => {
    if (!partner) return [];
    const partnerKinks = partner.kinks || [];
    if (partnerKinks.length === 0) return [];

    // Get user's "Looking for" kinks from localStorage (persisted)
    let lookingForKinks = [];
    try {
      const savedFilters = localStorage.getItem('brozr_match_filters');
      if (savedFilters) {
        lookingForKinks = JSON.parse(savedFilters).kinks || [];
      }
    } catch (e) {
      console.error('Error parsing match filters:', e);
    }
    // Fallback: si pas de filtre kinks, comparer avec les kinks du profil user
    if (lookingForKinks.length === 0) {
      lookingForKinks = user?.kinks || [];
      if (lookingForKinks.length === 0) {
        try { lookingForKinks = JSON.parse(localStorage.getItem('brozr_user') || '{}').kinks || []; } catch (e) {}
      }
    }
    if (lookingForKinks.length === 0) return [];
    const lookingForSet = new Set(lookingForKinks);
    return partnerKinks.filter(k => lookingForSet.has(k));
  }, [partner, user]);

  // Fetch unread notification count
  const fetchUnreadCount = useCallback(async () => {
    try {
      const res = await fetch(`${API_URL}/api/notifications/unread-count`, {
        headers: { 'Authorization': `Bearer ${token}` }
      });
      if (res.ok) {
        const json = await res.json();
        const data = json.data || json;
        setUnreadCount(data.count || 0);
      }
    } catch (err) {
      console.error('Failed to fetch unread count:', err);
    }
  }, [token]);

  // Fetch notifications list
  const fetchNotifications = useCallback(async () => {
    try {
      const res = await fetch(`${API_URL}/api/notifications`, {
        headers: { 'Authorization': `Bearer ${token}` }
      });
      if (res.ok) {
        const json = await res.json();
        const notifs = json.data || json;
        setNotifications(Array.isArray(notifs) ? notifs : []);
      }
    } catch (err) {
      console.error('Failed to fetch notifications:', err);
    }
  }, [token]);

  // Mark all notifications as read
  const markNotificationsRead = useCallback(async () => {
    try {
      await fetch(`${API_URL}/api/notifications/mark-all-read`, {
        method: 'POST',
        headers: { 'Authorization': `Bearer ${token}` }
      });
      setUnreadCount(0);
    } catch (err) {
      console.error('Failed to mark notifications as read:', err);
    }
  }, [token]);

  // Fetch notification count on mount
  useEffect(() => {
    if (token) {
      fetchUnreadCount();
      const interval = setInterval(fetchUnreadCount, 10000); // Poll every 10s
      return () => clearInterval(interval);
    }
  }, [token, fetchUnreadCount]);

  // Get partner country flag
  const getPartnerFlag = useCallback(() => {
    if (!partner) return '🌍';
    const code = partner.country || partner.detected_country;
    if (!code) return '🌍';
    const mappedCode = code === 'GB' ? 'UK' : code;
    return countryFlags[mappedCode] || '🌍';
  }, [partner]);

  // Get partner distance
  const getPartnerDistance = useCallback(() => {
    if (!partner) return null;
    // Distance should come from the backend match calculation
    if (partner.distance !== undefined && partner.distance !== null) {
      return Math.round(partner.distance);
    }
    return null;
  }, [partner]);

  // Setup local video stream - called ONCE at component mount, recreated on mobile per match
  const setupLocalStream = async () => {
    try {
      let stream;
      try {
        stream = await navigator.mediaDevices.getUserMedia({
          video: { facingMode, width: { ideal: 1280 }, height: { ideal: 720 } },
          audio: true
        });
      } catch (firstErr) {
        // Retry with relaxed constraints on iOS OverconstrainedError / NotReadableError
        if (firstErr.name === 'OverconstrainedError' || firstErr.name === 'NotReadableError') {
          stream = await navigator.mediaDevices.getUserMedia({
            video: { facingMode },
            audio: true
          });
        } else {
          throw firstErr;
        }
      }

      localStreamRef.current = stream;

      // Immediately attach to video element
      if (localVideoRef.current) {
        localVideoRef.current.setAttribute('playsinline', '');
        localVideoRef.current.srcObject = stream;
        setLocalStreamReady(true);
      }
      return stream;
    } catch (err) {
      console.error('Camera error:', err);
      if (err.name === 'NotAllowedError') {
        setError('Autorise ta camera dans les reglages');
      } else if (err.name === 'NotReadableError') {
        setError('Camera deja utilisee par une autre app');
      } else {
        setError('Erreur camera - rafraichis la page');
      }
      return null;
    }
  };

  // Check if local stream is still active (all tracks are "live")
  const isLocalStreamActive = () => {
    if (!localStreamRef.current) return false;
    const tracks = localStreamRef.current.getTracks();
    if (tracks.length === 0) return false;
    return tracks.every(track => track.readyState === 'live');
  };

  // Get a valid local stream - reuse existing if still active
  const getValidLocalStream = async () => {
    if (isLocalStreamActive()) return localStreamRef.current;
    return await setupLocalStream();
  };

  // Alias for backward compatibility
  const ensureLocalStream = getValidLocalStream;

  // Stream health monitoring - detects frozen/black video
  const startStreamHealthCheck = (matchId) => {
    if (healthCheckRef.current) clearInterval(healthCheckRef.current);
    let lastByteCount = 0;
    let noDataCount = 0;

    healthCheckRef.current = setInterval(async () => {
      if (currentMatchIdRef.current !== matchId) {
        clearInterval(healthCheckRef.current);
        healthCheckRef.current = null;
        return;
      }
      const pc = peerConnectionRef.current;
      if (!pc || pc.connectionState !== 'connected') return;

      try {
        const stats = await pc.getStats();
        stats.forEach(report => {
          if (report.type === 'inbound-rtp' && report.kind === 'video') {
            if (report.bytesReceived === lastByteCount) {
              noDataCount++;
              if (noDataCount >= 4) {
                clearInterval(healthCheckRef.current);
                healthCheckRef.current = null;
              }
            } else {
              noDataCount = 0;
              lastByteCount = report.bytesReceived;
            }
          }
        });
      } catch (e) {
        // PC might have been closed
      }
    }, 3000);
  };

  // Helper function to create peer connection for direct calls
  const createPeerConnectionForDirect = async (socket, matchId, isInitiator, stream) => {
    const pc = new RTCPeerConnection(ICE_CONFIG);
    peerConnectionRef.current = pc;
    pc._matchId = matchId;
    
    // Add tracks
    stream.getTracks().forEach(track => {
      pc.addTrack(track, stream);
    });
    
    pc.ontrack = (event) => {
      if (remoteVideoRef.current && event.streams[0]) {
        remoteVideoRef.current.srcObject = event.streams[0];
        setTimeout(() => {
          remoteVideoRef.current?.play().catch(console.error);
          setConnectionState('connected');
        }, 100);
      }
    };
    
    pc.onicecandidate = (event) => {
      if (event.candidate) {
        socket.emit('webrtc-signal', {
          type: 'ice-candidate',
          candidate: event.candidate,
          match_id: matchId
        });
      }
    };

    pc.onconnectionstatechange = () => {
      if (pc.connectionState === 'failed' || pc.connectionState === 'disconnected') {
        setConnectionState('searching');
      }
    };

    if (isInitiator) {
      // Create and send offer
      const offer = await pc.createOffer();
      await pc.setLocalDescription(offer);
      socket.emit('webrtc-signal', { type: 'offer', sdp: offer.sdp, match_id: matchId });
    }
  };
  
  // Signaling is handled via the multiplexed 'webrtc-signal' event.
  // See the webrtc-signal listener in the direct call section and regular matching section.

  // Re-attach stream to video element when ref becomes available
  useEffect(() => {
    if (localVideoRef.current && localStreamRef.current) {
      if (!localVideoRef.current.srcObject) {
        localVideoRef.current.srcObject = localStreamRef.current;
      }
    }
  });

  useEffect(() => {
    if (!token) {
      navigate('/login');
      return;
    }

    let isActive = true;
    let directPollInterval = null;

    // Pre-load following list to check follow status on match
    fetch(`${process.env.REACT_APP_BACKEND_URL}/api/users/following`, {
      headers: { 'Authorization': `Bearer ${token}` }
    }).then(r => r.ok ? r.json() : null).then(json => {
      if (json) {
        const bros = json.data || json;
        followingIdsRef.current = new Set((Array.isArray(bros) ? bros : []).map(b => b.id));
      }
    }).catch(() => {});

    const initializeCall = async () => {
      const stream = await setupLocalStream();
      if (!stream || !isActive) return;

      const SOCKET_URL = process.env.REACT_APP_SOCKET_URL;

      // Check if this is a direct Live Cam call
      const directCall = location.state?.directCall;
      const partnerId = location.state?.partnerId;
      const partnerName = location.state?.partnerName;

      // Create socket connection (same for direct and regular)
      const socket = io(SOCKET_URL, {
        auth: { token },
        query: { token },
        transports: ['websocket', 'polling'],
        reconnection: true,
        reconnectionAttempts: 10,
        reconnectionDelay: 1000,
        reconnectionDelayMax: 5000,
        timeout: 20000,
        autoConnect: false  // Don't auto-connect — we call socket.connect() after listeners
      });
      socketRef.current = socket;
      
      if (directCall && partnerId) {
        setConnectionState('searching');
        setPartner({ display_name: partnerName, id: partnerId });
        
        // FIRST: Attach ALL listeners before connecting
        socket.on('connect', () => {});
        
        // TODO: direct_match_ready / direct_match_confirmed are not supported by the backend.
        // Direct calls should use the REST livecam API (POST /livecam/request, PUT /livecam/:id/respond)
        // to create a match, then join the resulting room_id via the regular socket flow.
        // The code below is commented out until this flow is reimplemented.

        // Wait for server confirmation that we're registered
        socket.on('connected', (data) => {
          // TODO: Replace with REST-based livecam flow instead of unsupported socket event
          // socket.emit('direct_match_ready', { partner_id: partnerId });
        });

        // TODO: Replace direct_match_confirmed with a mechanism that uses the livecam REST API
        // to get a match_id/room_id, then setup WebRTC using that match_id.
        // For now, the direct call WebRTC setup is triggered when match-found is received
        // after joining the room via the livecam API flow.

        /* direct_match_confirmed listener removed - backend does not emit this event
        socket.on('direct_match_confirmed', async (data) => {
          ...
        });
        */

        // Handle webrtc-signal for direct calls (offer/answer/ice-candidate multiplexed)
        socket.on('webrtc-signal', async (data) => {
          const pc = peerConnectionRef.current;
          if (!pc || (data.match_id && data.match_id !== currentMatchIdRef.current)) return;

          try {
            if (data.type === 'offer') {
              await pc.setRemoteDescription(new RTCSessionDescription({ type: 'offer', sdp: data.sdp }));
              const answer = await pc.createAnswer();
              await pc.setLocalDescription(answer);
              socket.emit('webrtc-signal', { type: 'answer', sdp: answer.sdp, match_id: currentMatchIdRef.current });
            } else if (data.type === 'answer') {
              await pc.setRemoteDescription(new RTCSessionDescription({ type: 'answer', sdp: data.sdp }));
            } else if (data.type === 'ice-candidate' && data.candidate) {
              if (pc.remoteDescription) {
                await pc.addIceCandidate(new RTCIceCandidate(data.candidate));
              }
            }
          } catch (err) {
            console.error('Direct call signal error:', err);
          }
        });
        
        socket.on('partner-disconnected', async () => {
          currentMatchIdRef.current = null;
          if (clonedStreamRef.current) {
            clonedStreamRef.current.getTracks().forEach(track => track.stop());
            clonedStreamRef.current = null;
          }
          if (peerConnectionRef.current) {
            peerConnectionRef.current.ontrack = null;
            peerConnectionRef.current.onicecandidate = null;
            peerConnectionRef.current.onconnectionstatechange = null;
            peerConnectionRef.current.oniceconnectionstatechange = null;
            peerConnectionRef.current.close();
            peerConnectionRef.current = null;
          }
          if (remoteVideoRef.current) remoteVideoRef.current.srcObject = null;
          setPartner(null);
          setLiked(false);
          setChatMessages([]);
          setFollowSent(false);
          setFollowStatus(null);
          setIsMutualFollow(false);
          setShowChatInput(false);
          setConnectionState('searching');
          // Rediriger vers le flux normal
          socket.disconnect();
          navigate('/video-call', { replace: true, state: null });
        });

        socket.on('partner-left', async () => {
          currentMatchIdRef.current = null;
          if (clonedStreamRef.current) {
            clonedStreamRef.current.getTracks().forEach(track => track.stop());
            clonedStreamRef.current = null;
          }
          if (peerConnectionRef.current) {
            peerConnectionRef.current.ontrack = null;
            peerConnectionRef.current.onicecandidate = null;
            peerConnectionRef.current.onconnectionstatechange = null;
            peerConnectionRef.current.oniceconnectionstatechange = null;
            peerConnectionRef.current.close();
            peerConnectionRef.current = null;
          }
          if (remoteVideoRef.current) remoteVideoRef.current.srcObject = null;
          setPartner(null);
          setLiked(false);
          setChatMessages([]);
          setFollowSent(false);
          setFollowStatus(null);
          setIsMutualFollow(false);
          setShowChatInput(false);
          setConnectionState('searching');
          // Rediriger vers le flux normal
          socket.disconnect();
          navigate('/video-call', { replace: true, state: null });
        });

        socket.on('disconnect', () => {});

        // NOW connect after all listeners are attached
        socket.connect();
        
        return; // Exit early for direct call mode
      }

      // Regular matching flow - also need to attach listeners before connecting
      socket.on('connect', () => {
        if (isActive) {
          setConnectionState('searching');
          // Build join-queue payload matching backend JoinQueuePayload
          const joinPayload = {
            display_name: user?.display_name || user?.username || 'Anonymous',
            bio: user?.bio || null,
            age: user?.age || 18,
            country: user?.country || null,
            kinks: user?.kinks || [],
            profile_photo_url: user?.profile_photo_url || null,
            filters: {
              country: filters.country === 'ALL' ? null : (filters.country || null),
              age_min: filters.ageMin || filters.age_min || null,
              age_max: filters.ageMax || filters.age_max || null,
              kinks: filters.kinks || [],
            },
          };
          socket.emit('join-queue', joinPayload);
        }
      });

      socket.on('connect_error', (err) => {
        console.error('Socket connect_error:', err.message);
        if (isActive && !err.message.includes('timeout')) {
          setError('Erreur de connexion');
        }
      });

      socket.on('error', () => {});

      socket.on('searching', (data) => {
        if (isActive) {
          setQueueSize(data.queue_size || 0);
          setConnectionState('searching');
        }
      });

      socket.on('match-found', async (data) => {
        const matchId = data.match_id;
        if (!isActive) return;
        
        // Store the match ID - this is the KEY to filtering stale signals
        currentMatchIdRef.current = matchId;
        
        // Clear buffered ICE candidates from previous match
        pendingIceCandidatesRef.current = [];
        if (healthCheckRef.current) { clearInterval(healthCheckRef.current); healthCheckRef.current = null; }
        if (clonedStreamRef.current) {
          clonedStreamRef.current.getTracks().forEach(t => { t.stop(); t.onended = null; });
          clonedStreamRef.current = null;
        }

        // Clean close any existing peer connection
        if (peerConnectionRef.current) {
          // Remove all senders first
          try {
            peerConnectionRef.current.getSenders().forEach(sender => {
              peerConnectionRef.current.removeTrack(sender);
            });
          } catch (e) {}
          
          peerConnectionRef.current.ontrack = null;
          peerConnectionRef.current.onicecandidate = null;
          peerConnectionRef.current.onconnectionstatechange = null;
          peerConnectionRef.current.oniceconnectionstatechange = null;
          peerConnectionRef.current.close();
          peerConnectionRef.current = null;
        }
        // Give mobile browsers time to release WebRTC resources
        await new Promise(resolve => setTimeout(resolve, isMobile ? 500 : 100));

        if (remoteVideoRef.current) {
          remoteVideoRef.current.srcObject = null;
        }
        
        setPartner(data.partner);
        setConnectionState('connecting');
        
        // Set like/follow status — check if already liked this partner
        const partnerId = data.partner?.user_id || data.partner?.id;
        if (partnerId) {
          fetch(`${process.env.REACT_APP_BACKEND_URL}/api/users/likes/check/${partnerId}`, {
            headers: { 'Authorization': `Bearer ${token}` }
          }).then(r => r.ok ? r.json() : null).then(json => {
            if (json) {
              const d = json.data || json;
              setLiked(!!d.already_liked);
            }
          }).catch(() => setLiked(false));
        } else {
          setLiked(false);
        }
        if (partnerId && followingIdsRef.current.has(partnerId)) {
          setFollowStatus('accepted');
          setFollowSent(true);
        } else {
          setFollowStatus(null);
          setFollowSent(false);
          setIsMutualFollow(false);
        }
        setChatMessages([]);
        setShowChatInput(false);
        setShowIncomingFollowRequest(false);
        setFollowRequester(null);
        
        // Reuse existing stream if valid — always recreate on mobile to prevent black screen
        let activeStream = localStreamRef.current;

        const hasValidStream = activeStream &&
          activeStream.getTracks().length > 0 &&
          activeStream.getTracks().every(track => track.readyState === 'live');
        const shouldRecreate = isMobile; // Always recreate on mobile

        if (hasValidStream && !shouldRecreate) {
          // Reuse existing stream
        } else {
          if (localStreamRef.current) {
            localStreamRef.current.getTracks().forEach(track => track.stop());
            localStreamRef.current = null;
          }
          if (clonedStreamRef.current) {
            clonedStreamRef.current.getTracks().forEach(track => track.stop());
            clonedStreamRef.current = null;
          }
          activeStream = await setupLocalStream();
          if (!activeStream) {
            console.error('❌ Failed to create local stream');
            setError('Erreur caméra - rafraîchis la page');
            return;
          }
        }
        // Create a cloned stream for the peer connection
        // This protects the original tracks from being affected when PC is closed
        // Stop previous cloned tracks before creating new ones
        if (clonedStreamRef.current) {
          clonedStreamRef.current.getTracks().forEach(track => track.stop());
        }
        const clonedStream = new MediaStream(activeStream.getTracks().map(track => track.clone()));
        clonedStreamRef.current = clonedStream;
        
        // Re-attach local stream to video element
        if (localVideoRef.current) {
          localVideoRef.current.srcObject = activeStream;
        }
        
        // Check match ID is still current (could have changed during async)
        if (currentMatchIdRef.current !== matchId) return;
        
        const pc = new RTCPeerConnection(ICE_CONFIG);
        peerConnectionRef.current = pc;
        pc._matchId = matchId;
        
        // Add tracks from the fresh stream
        clonedStream.getTracks().forEach(track => {
          pc.addTrack(track, clonedStream);
        });
        
        pc.ontrack = (event) => {
          const stream = event.streams[0];
          if (currentMatchIdRef.current !== matchId) return;

          if (remoteVideoRef.current && stream) {
            if (remoteVideoRef.current.srcObject !== stream) {
              remoteVideoRef.current.setAttribute('playsinline', '');
              remoteVideoRef.current.srcObject = stream;
              
              // Wait a moment for all tracks to be ready, then play
              setTimeout(() => {
                if (remoteVideoRef.current && currentMatchIdRef.current === matchId) {
                  remoteVideoRef.current.play()
                    .then(() => {
                      setConnectionState('connected');
                    })
                    .catch(() => {
                      setConnectionState('connected');
                    });
                }
              }, 100);
            } else {
              setConnectionState('connected');
            }
          }
        };
        
        pc.onicecandidate = (event) => {
          // Verify this is still the current match
          if (pc._matchId !== currentMatchIdRef.current) return;
          if (event.candidate) {
            // Include match_id in the signal for filtering on the other side
            socket.emit('webrtc-signal', { 
              type: 'ice-candidate', 
              candidate: event.candidate,
              match_id: matchId
            });
          }
        };
        
        pc.onicegatheringstatechange = () => {};

        pc.onconnectionstatechange = () => {
          if (pc._matchId !== currentMatchIdRef.current) return;
          if (isSwitchingCameraRef.current) return;
          if (pc.connectionState === 'connected') {
            setConnectionState('connected');
            // Start stream health monitoring
            startStreamHealthCheck(matchId);
          }
          // Only handle 'failed', NOT 'disconnected' (can be temporary)
          else if (pc.connectionState === 'failed') {
            // Try ICE restart before giving up
            if (data.is_initiator) {
              pc.restartIce();
            }
          }
        };
        
        pc.oniceconnectionstatechange = () => {
          if (pc._matchId !== currentMatchIdRef.current) return;
          if (isSwitchingCameraRef.current) return;
          if (pc.iceConnectionState === 'failed') {
            handleConnectionLost(socket);
          }
        };
        
        // Helper to handle connection lost
        const handleConnectionLost = async (socket) => {
          // Clear match ID to prevent any more signals for this connection
          currentMatchIdRef.current = null;
          peerConnectionRef.current?.close();
          peerConnectionRef.current = null;
          if (remoteVideoRef.current) remoteVideoRef.current.pause(); // freeze last frame instead of black
          setPartner(null);
          setLiked(false);
          setChatMessages([]);
          setFollowSent(false);
          setFollowStatus(null);
          setIsMutualFollow(false);
          setShowChatInput(false);
          setConnectionState('searching');
          // Ensure stream is still active before rejoining queue
          await ensureLocalStream();
          socket.emit('join-queue', {
            display_name: user?.display_name || user?.username || 'Anonymous',
            bio: user?.bio || null,
            age: user?.age || 18,
            country: user?.country || null,
            kinks: user?.kinks || [],
            profile_photo_url: user?.profile_photo_url || null,
            filters: {
              country: filters.country === 'ALL' ? null : (filters.country || null),
              age_min: filters.ageMin || filters.age_min || null,
              age_max: filters.ageMax || filters.age_max || null,
              kinks: filters.kinks || [],
            },
          });
        };
        
        if (data.is_initiator) {
          // Small delay to ensure both peers are ready
          await new Promise(r => setTimeout(r, 500));
          // Verify match is still current
          if (pc._matchId !== currentMatchIdRef.current) return;
          try {
            const offer = await pc.createOffer();
            await pc.setLocalDescription(offer);
            // Include match_id in the signal
            socket.emit('webrtc-signal', { type: 'offer', sdp: offer.sdp, match_id: matchId });
          } catch (err) {
            console.error('Offer error:', err);
          }
        }
      });

      socket.on('webrtc-signal', async (data) => {
        const pc = peerConnectionRef.current;
        if (!pc) return;
        if (data.match_id && data.match_id !== currentMatchIdRef.current) return;
        if (pc.signalingState === 'closed') return;
        
        try {
          if (data.type === 'offer') {
            // Only accept offer if we're in stable state
            if (pc.signalingState !== 'stable') {
              return;
            }
            // Clear any pending candidates from previous attempt
            pendingIceCandidatesRef.current = [];
            
            await pc.setRemoteDescription(new RTCSessionDescription({ type: 'offer', sdp: data.sdp }));
            
            for (const candidate of pendingIceCandidatesRef.current) {
              try { await pc.addIceCandidate(new RTCIceCandidate(candidate)); } catch (e) {}
            }
            pendingIceCandidatesRef.current = [];

            const answer = await pc.createAnswer();
            await pc.setLocalDescription(answer);
            socket.emit('webrtc-signal', { type: 'answer', sdp: answer.sdp, match_id: currentMatchIdRef.current });
          } else if (data.type === 'answer') {
            // Only accept answer if we have a local offer
            if (pc.signalingState !== 'have-local-offer') {
              return;
            }
            await pc.setRemoteDescription(new RTCSessionDescription({ type: 'answer', sdp: data.sdp }));
            for (const candidate of pendingIceCandidatesRef.current) {
              try { await pc.addIceCandidate(new RTCIceCandidate(candidate)); } catch (e) {}
            }
            pendingIceCandidatesRef.current = [];
          } else if (data.type === 'ice-candidate' && data.candidate) {
            // Add ICE candidates - buffer if remote description not set yet
            if (pc.remoteDescription) {
              await pc.addIceCandidate(new RTCIceCandidate(data.candidate));
            } else {
              pendingIceCandidatesRef.current.push(data.candidate);
            }
          }
        } catch (err) {
          console.error('Signal error:', err, 'State:', pc.signalingState);
        }
      });

      socket.on('partner-disconnected', async () => {
        if (isActive) {
          currentMatchIdRef.current = null;
          if (clonedStreamRef.current) {
            clonedStreamRef.current.getTracks().forEach(t => { t.stop(); t.onended = null; });
            clonedStreamRef.current = null;
          }
          if (peerConnectionRef.current) {
            peerConnectionRef.current.ontrack = null;
            peerConnectionRef.current.onicecandidate = null;
            peerConnectionRef.current.onconnectionstatechange = null;
            peerConnectionRef.current.oniceconnectionstatechange = null;
            try {
              peerConnectionRef.current.getSenders().forEach(s => {
                try { peerConnectionRef.current.removeTrack(s); } catch(e) {}
              });
            } catch(e) {}
            peerConnectionRef.current.close();
            peerConnectionRef.current = null;
          }
          pendingIceCandidatesRef.current = [];
          await new Promise(resolve => setTimeout(resolve, isMobile ? 500 : 100));
          if (remoteVideoRef.current) remoteVideoRef.current.pause(); // freeze last frame instead of black

          setPartner(null);
          setLiked(false);
          setChatMessages([]);
          setFollowSent(false);
          setFollowStatus(null);
          setIsMutualFollow(false);
          setShowChatInput(false);
          setConnectionState('searching');

          await ensureLocalStream();
          socket.emit('join-queue', {
            display_name: user?.display_name || user?.username || 'Anonymous',
            bio: user?.bio || null,
            age: user?.age || 18,
            country: user?.country || null,
            kinks: user?.kinks || [],
            profile_photo_url: user?.profile_photo_url || null,
            filters: {
              country: filters.country === 'ALL' ? null : (filters.country || null),
              age_min: filters.ageMin || filters.age_min || null,
              age_max: filters.ageMax || filters.age_max || null,
              kinks: filters.kinks || [],
            },
          });
        }
      });

      socket.on('partner-left', async () => {
        if (isActive) {
          currentMatchIdRef.current = null;
          if (clonedStreamRef.current) {
            clonedStreamRef.current.getTracks().forEach(t => { t.stop(); t.onended = null; });
            clonedStreamRef.current = null;
          }
          if (peerConnectionRef.current) {
            peerConnectionRef.current.ontrack = null;
            peerConnectionRef.current.onicecandidate = null;
            peerConnectionRef.current.onconnectionstatechange = null;
            peerConnectionRef.current.oniceconnectionstatechange = null;
            try {
              peerConnectionRef.current.getSenders().forEach(s => {
                try { peerConnectionRef.current.removeTrack(s); } catch(e) {}
              });
            } catch(e) {}
            peerConnectionRef.current.close();
            peerConnectionRef.current = null;
          }
          pendingIceCandidatesRef.current = [];
          await new Promise(resolve => setTimeout(resolve, isMobile ? 500 : 100));
          if (remoteVideoRef.current) remoteVideoRef.current.pause(); // freeze last frame instead of black

          setPartner(null);
          setLiked(false);
          setChatMessages([]);
          setFollowSent(false);
          setFollowStatus(null);
          setIsMutualFollow(false);
          setShowChatInput(false);
          setConnectionState('searching');

          await ensureLocalStream();
          socket.emit('join-queue', {
            display_name: user?.display_name || user?.username || 'Anonymous',
            bio: user?.bio || null,
            age: user?.age || 18,
            country: user?.country || null,
            kinks: user?.kinks || [],
            profile_photo_url: user?.profile_photo_url || null,
            filters: {
              country: filters.country === 'ALL' ? null : (filters.country || null),
              age_min: filters.ageMin || filters.age_min || null,
              age_max: filters.ageMax || filters.age_max || null,
              kinks: filters.kinks || [],
            },
          });
        }
      });

      socket.on('like-received', (data) => {
        // Use total from server if provided, otherwise increment
        if (data && data.total_likes !== undefined) {
          setLikesReceived(data.total_likes);
        } else {
          setLikesReceived(prev => prev + 1);
        }
        setShowHearts(true);
        setTimeout(() => setShowHearts(false), 3000);
      });
      
      socket.on('like-sent', (data) => {
        // Backend confirms our like was sent successfully (payload: { target_id })
        setLiked(true);
        setShowHearts(true);
        setTimeout(() => setShowHearts(false), 3000);
      });

      socket.on('chat-message', (data) => {
        const msgId = Date.now();
        setChatMessages(prev => [...prev, {
          id: msgId,
          message: data.content,
          fromPartner: true,
          timestamp: data.timestamp
        }]);
        // Auto-remove message after 30 seconds
        setTimeout(() => {
          setChatMessages(prev => prev.filter(m => m.id !== msgId));
        }, 30000);
      });

      // Handle connection rejected (account already connected elsewhere)
      socket.on('connection-rejected', (data) => {
        if (isActive) {
          setError(data.message || 'Ce compte est déjà connecté ailleurs');
        }
      });

      // Handle force disconnect (new session opened elsewhere)
      socket.on('force-disconnect', (data) => {
        if (isActive) {
          setError(data.message || 'Connecté depuis un autre appareil');
          navigate('/live-prematch');
        }
      });

      // Handle incoming follow request from partner
      socket.on('follow-request', (data) => {
        if (isActive) {
          setFollowRequester({
            id: data.from_user_id,
            name: data.from_user_name,
            photo: data.from_user_photo
          });
          setShowIncomingFollowRequest(true);
        }
      });

      // Handle follow response from partner - both users get mutual follow
      socket.on('follow-response', (data) => {
        if (isActive) {
          if (data.accepted) {
            setFollowStatus('accepted');
            setIsMutualFollow(true);
            // Update pre-loaded following set
            const pid = data.target_id || data.partner_id;
            if (pid) followingIdsRef.current.add(pid);
          } else {
            setFollowStatus('refused');
          }
        }
      });

      // Handle when partner unfollows us
      socket.on('bro-removed', (data) => {
        if (isActive) {
          // Always update follow status when we receive bro-removed
          // The backend only sends this to the correct user
          setFollowStatus(null);
          setFollowSent(false);
          setIsMutualFollow(false);
        }
      });
      
      socket.connect();
    };

    initializeCall();

    return () => {
      isActive = false;
      if (directPollInterval) clearInterval(directPollInterval);
      if (healthCheckRef.current) { clearInterval(healthCheckRef.current); healthCheckRef.current = null; }
      socketRef.current?.disconnect();
      if (clonedStreamRef.current) {
        clonedStreamRef.current.getTracks().forEach(t => { t.stop(); t.onended = null; });
        clonedStreamRef.current = null;
      }
      peerConnectionRef.current?.close();
      peerConnectionRef.current = null;
      localStreamRef.current?.getTracks().forEach(t => t.stop());
      localStreamRef.current = null;
    };
  }, [token, navigate, filters, location.state]); // Added location.state for direct call mode

  // Handle Like with animation - improved with visual feedback
  const handleLike = () => {
    if (!liked && partner && socketRef.current?.connected) {
      socketRef.current.emit('send-like', { match_id: currentMatchIdRef.current });
    }
  };

  // Handle Follow - send request to partner
  const handleSendFollowRequest = () => {
    if (!followSent && partner && socketRef.current?.connected) {
      setFollowSent(true);
      setFollowStatus('pending');
      socketRef.current.emit('send-follow-request', { target_id: partner?.user_id || partner?.id });
      // No toast - silent action
    }
  };

  // Handle click on "Suivi" button - show unfollow confirmation
  const handleFollowButtonClick = () => {
    if (followStatus === 'accepted') {
      // Already following - show unfollow confirmation
      setShowUnfollowConfirm(true);
    } else {
      // Not following yet - send follow request
      handleSendFollowRequest();
    }
  };

  // Handle unfollow confirmation
  const handleUnfollow = async () => {
    if (partner && socketRef.current?.connected) {
      try {
        const token = localStorage.getItem('brozr_token');
        await fetch(`${process.env.REACT_APP_BACKEND_URL}/api/follows/${partner.id}`, {
          method: 'DELETE',
          headers: { 'Authorization': `Bearer ${token}` }
        });
        setFollowStatus(null);
        setFollowSent(false);
        setIsMutualFollow(false);
        setShowUnfollowConfirm(false);
      } catch (err) {
        setShowUnfollowConfirm(false);
      }
    }
  };

  // Handle respond to incoming follow request - both users become mutual follows
  const handleRespondFollowRequest = (accepted) => {
    socketRef.current?.emit('respond-follow-request', {
      follower_id: followRequester?.id,
      accepted: accepted,
    });
    setShowIncomingFollowRequest(false);
    if (accepted) {
      // Both users are now following each other
      setFollowStatus('accepted');
      setIsMutualFollow(true);
      setFollowSent(true);
      // Update pre-loaded following set
      if (followRequester?.id) followingIdsRef.current.add(followRequester.id);
    }
  };

  // Handle Next - simple but with proper cleanup
  const handleNext = () => {
    // Check if this was a direct call - if so, we need to restart with regular flow
    const wasDirectCall = peerConnectionRef.current?._isDirectCall;
    
    // Clear match ID to invalidate any pending signals
    currentMatchIdRef.current = null;
    
    // Clean close of peer connection
    if (clonedStreamRef.current) {
      clonedStreamRef.current.getTracks().forEach(t => { t.stop(); t.onended = null; });
      clonedStreamRef.current = null;
    }

    if (peerConnectionRef.current) {
      peerConnectionRef.current.ontrack = null;
      peerConnectionRef.current.onicecandidate = null;
      peerConnectionRef.current.onconnectionstatechange = null;
      peerConnectionRef.current.oniceconnectionstatechange = null;
      try {
        peerConnectionRef.current.getSenders().forEach(s => {
          try { peerConnectionRef.current.removeTrack(s); } catch(e) {}
        });
      } catch(e) {}
      peerConnectionRef.current.close();
      peerConnectionRef.current = null;
    }
    pendingIceCandidatesRef.current = [];
    if (remoteVideoRef.current) remoteVideoRef.current.pause(); // freeze last frame instead of black
    if (healthCheckRef.current) { clearInterval(healthCheckRef.current); healthCheckRef.current = null; }

    // Keep local stream active for next match (don't stop tracks)
    // Stream will be reused in match-found handler
    
    // Reset state
    setPartner(null);
    setLiked(false);
    setChatMessages([]);
    setFollowSent(false);
    setFollowStatus(null);
    setIsMutualFollow(false);
    setShowIncomingFollowRequest(false);
    setFollowRequester(null);
    setShowChatInput(false);
    setSwipeProgress(0);
    setConnectionState('searching');
    
    // For direct calls, navigate to /live WITHOUT directCall state to force regular flow
    // This ensures all socket handlers are properly initialized
    if (wasDirectCall) {
      // Disconnect current socket
      socketRef.current?.disconnect();
      socketRef.current = null;
      // Navigate directly to /video-call to enter the regular matching queue
      navigate('/video-call', { replace: true, state: null });
    } else {
      // For regular matches, just emit next_match
      socketRef.current?.emit('next-match', {
        display_name: user?.display_name || user?.username || 'Anonymous',
        bio: user?.bio || null,
        age: user?.age || 18,
        country: user?.country || null,
        kinks: user?.kinks || [],
        profile_photo_url: user?.profile_photo_url || null,
        filters: {
          country: tempCountry === 'ALL' ? null : tempCountry,
          age_min: tempAgeMin || null,
          age_max: tempAgeMax || null,
          kinks: tempKinks || [],
        },
      });
    }
  };

  // Swipe handlers for mobile
  const handleTouchStart = (e) => {
    if (!isMobile || connectionState !== 'connected') return;
    swipeStartX.current = e.touches[0].clientX;
    swipeStartY.current = e.touches[0].clientY;
  };

  const handleTouchMove = (e) => {
    if (!isMobile || connectionState !== 'connected' || swipeStartX.current === 0) return;
    
    const currentX = e.touches[0].clientX;
    const currentY = e.touches[0].clientY;
    const diffX = currentX - swipeStartX.current;
    const diffY = Math.abs(currentY - swipeStartY.current);
    
    // Only track horizontal swipes (ignore vertical scrolling)
    if (diffY > Math.abs(diffX) * 0.5) return;
    
    // Calculate swipe progress (0-100) for visual feedback
    // Require at least 100px swipe to trigger
    const threshold = 150;
    if (diffX > 0) {
      const progress = Math.min((diffX / threshold) * 100, 100);
      setSwipeProgress(progress);
    }
  };

  const handleTouchEnd = () => {
    if (!isMobile || connectionState !== 'connected') return;
    
    // If swipe progress >= 100%, trigger next
    if (swipeProgress >= 100) {
      handleNext();
    } else {
      // Reset swipe progress with animation
      setSwipeProgress(0);
    }
    
    swipeStartX.current = 0;
    swipeStartY.current = 0;
  };

  // Handle End Session
  const handleEndSession = () => {
    socketRef.current?.emit('end-call', { match_id: currentMatchIdRef.current });
    navigate('/live-prematch');
  };

  // Switch camera - ONLY changes local preview, does NOT affect WebRTC connection
  const switchCamera = async () => {
    const newMode = facingMode === 'user' ? 'environment' : 'user';
    
    // Set flag to prevent disconnect handler from triggering
    isSwitchingCameraRef.current = true;
    
    try {
      // Get new video stream
      const newStream = await navigator.mediaDevices.getUserMedia({ 
        video: { facingMode: newMode, width: { ideal: 1280 }, height: { ideal: 720 } }, 
        audio: true
      });
      
      const newVideoTrack = newStream.getVideoTracks()[0];
      const newAudioTrack = newStream.getAudioTracks()[0];
      
      if (!newVideoTrack) {
        throw new Error('No video track obtained');
      }
      
      // Update local preview
      if (localVideoRef.current) {
        localVideoRef.current.srcObject = newStream;
      }
      
      // Update the track in peer connection so partner sees new camera
      if (peerConnectionRef.current) {
        const senders = peerConnectionRef.current.getSenders();
        
        // Replace video track
        const videoSender = senders.find(s => s.track?.kind === 'video');
        if (videoSender) {
          await videoSender.replaceTrack(newVideoTrack);
        }
        
        // Replace audio track if available
        if (newAudioTrack) {
          const audioSender = senders.find(s => s.track?.kind === 'audio');
          if (audioSender) {
            await audioSender.replaceTrack(newAudioTrack);
          }
        }
      }
      
      // Stop old stream tracks
      if (localStreamRef.current) {
        localStreamRef.current.getTracks().forEach(track => track.stop());
      }
      
      // Update refs
      localStreamRef.current = newStream;
      setFacingMode(newMode);
    } catch (err) {
      console.error('📸 Switch camera error:', err);
      toast.error('Impossible de changer de caméra');
    } finally {
      isSwitchingCameraRef.current = false;
    }
  };

  // Toggle kink in temp filters
  const toggleKink = (kink) => {
    setTempKinks(prev => prev.includes(kink) ? prev.filter(k => k !== kink) : [...prev, kink]);
  };

  // Save filters and persist to localStorage
  const saveFilters = () => {
    setActiveKinks([...tempKinks]);
    
    // Save to localStorage for persistence across sessions
    const filtersToSave = {
      country: tempCountry,
      minAge: tempAgeMin,
      maxAge: tempAgeMax,
      ageMin: tempAgeMin,
      ageMax: tempAgeMax,
      distance: tempDistance,
      kinks: tempKinks
    };
    localStorage.setItem('brozr_match_filters', JSON.stringify(filtersToSave));

    // Update local state for immediate UI reactivity
    setSavedFilters(filtersToSave);
    
    setShowFiltersOverlay(false);
  };

  // Toggle temp kink (for filter overlay)
  const toggleTempKink = (k) => {
    if (tempKinks.includes(k)) {
      setTempKinks(tempKinks.filter(x => x !== k));
    } else {
      setTempKinks([...tempKinks, k]);
    }
  };

  // Send chat message
  const sendChatMessage = () => {
    if (!chatMessage.trim()) {
      return;
    }
    
    const msgId = Date.now();
    const msg = chatMessage.trim();
    
    // Add to local messages immediately
    setChatMessages(prev => [...prev, {
      id: msgId,
      message: msg,
      fromPartner: false,
      timestamp: new Date().toISOString()
    }]);
    
    // Clear input
    setChatMessage('');
    
    // Send via socket if connected
    if (socketRef.current?.connected) {
      socketRef.current.emit('chat-message', { match_id: currentMatchIdRef.current, content: msg });
    }
    
    // Auto-remove message after 30 seconds
    setTimeout(() => {
      setChatMessages(prev => prev.filter(m => m.id !== msgId));
    }, 30000);
  };

  // Render kink categories for filters (avoiding babel bug)
  const renderKinkCategories = () => {
    const sections = [];
    for (let i = 0; i < kinkCategories.length; i++) {
      const cat = kinkCategories[i];
      const chips = [];
      for (let j = 0; j < cat.items.length; j++) {
        const k = cat.items[j];
        chips.push(<KinkChip key={k} label={k} selected={tempKinks.includes(k)} onToggle={() => toggleTempKink(k)} />);
      }
      sections.push(
        <div key={cat.cat} className="mb-4">
          <p className="text-white/50 text-xs uppercase tracking-wide mb-2">{cat.emoji} {cat.cat}</p>
          <div className="flex flex-wrap gap-2">{chips}</div>
        </div>
      );
    }
    return sections;
  };

  // Render partner kinks (avoiding babel bug) - matching kinks first, max 3 on one line!
  const renderPartnerKinks = () => {
    if (!partner) return null;
    const partnerKinks = partner.kinks || [];
    if (partnerKinks.length === 0) return null;
    
    const matchingKinks = getMatchingKinks();
    const hasComparisonKinks = matchingKinks.length > 0 || (() => {
      try {
        const savedFilters = localStorage.getItem('brozr_match_filters');
        if (savedFilters && (JSON.parse(savedFilters).kinks || []).length > 0) return true;
      } catch (e) {}
      // Fallback: check user profile kinks
      if (user?.kinks?.length > 0) return true;
      try { if ((JSON.parse(localStorage.getItem('brozr_user') || '{}').kinks || []).length > 0) return true; } catch (e) {}
      return false;
    })();

    // Sort: matching kinks first, then others
    const sortedKinks = [];
    const otherKinks = [];
    for (let i = 0; i < partnerKinks.length; i++) {
      if (matchingKinks.includes(partnerKinks[i])) {
        sortedKinks.push(partnerKinks[i]);
      } else {
        otherKinks.push(partnerKinks[i]);
      }
    }
    // Combine: matching first, then others
    const allSorted = sortedKinks.concat(otherKinks);

    // Show max 3 kinks on one line
    const kinksToShow = allSorted.slice(0, 3);
    const remainingCount = allSorted.length - 3;

    const elements = [];
    for (let i = 0; i < kinksToShow.length; i++) {
      const kink = kinksToShow[i];
      const isMatching = matchingKinks.includes(kink);

      // Style: Si pas de kinks de comparaison, affichage normal. Sinon: correspondants en blanc, autres floutés
      let className;
      if (!hasComparisonKinks) {
        className = 'px-2 py-0.5 rounded-full text-[10px] whitespace-nowrap bg-white/10 text-white border border-white/20';
      } else if (isMatching) {
        className = 'px-2 py-0.5 rounded-full text-[10px] whitespace-nowrap bg-white text-black font-semibold';
      } else {
        className = 'px-2 py-0.5 rounded-full text-[10px] whitespace-nowrap bg-white/5 text-white/50';
      }

      elements.push(
        <span key={`kink-${i}`} className={className}>
          {kink}
        </span>
      );
    }
    
    if (remainingCount > 0) {
      elements.push(
        <button 
          key="more"
          onClick={() => setShowKinksOverlay(true)} 
          className="text-white font-bold text-[10px] hover:underline ml-1"
        >
          +{remainingCount}
        </button>
      );
    }
    
    return elements;
  };

  // Render all partner kinks in overlay (avoiding babel bug) - common kinks first!
  const renderAllPartnerKinks = () => {
    if (!partner) return null;
    const partnerKinks = partner.kinks || [];
    if (partnerKinks.length === 0) return <p className="text-white/50 text-sm">Aucun kink sélectionné</p>;
    
    const matchingKinks = getMatchingKinks();
    const hasComparisonKinks = matchingKinks.length > 0 || (() => {
      try {
        const savedFilters = localStorage.getItem('brozr_match_filters');
        if (savedFilters && (JSON.parse(savedFilters).kinks || []).length > 0) return true;
      } catch (e) {}
      if (user?.kinks?.length > 0) return true;
      try { if ((JSON.parse(localStorage.getItem('brozr_user') || '{}').kinks || []).length > 0) return true; } catch (e) {}
      return false;
    })();

    // Sort: matching kinks first, then others
    const sortedKinks = [];
    const otherKinks = [];
    for (let i = 0; i < partnerKinks.length; i++) {
      if (matchingKinks.includes(partnerKinks[i])) {
        sortedKinks.push(partnerKinks[i]);
      } else {
        otherKinks.push(partnerKinks[i]);
      }
    }
    const allSorted = sortedKinks.concat(otherKinks);

    const elements = [];
    for (let i = 0; i < allSorted.length; i++) {
      const kink = allSorted[i];
      const isMatching = matchingKinks.includes(kink);

      let className;
      if (!hasComparisonKinks) {
        className = 'px-3 py-1.5 rounded-full text-sm bg-white/10 text-white';
      } else if (isMatching) {
        className = 'px-3 py-1.5 rounded-full text-sm bg-white text-black font-medium';
      } else {
        className = 'px-3 py-1.5 rounded-full text-sm bg-white/5 text-white/50';
      }
      
      elements.push(
        <span key={`kink-overlay-${i}`} className={className}>
          {kink}
        </span>
      );
    }
    
    return elements;
  };

  const matchingKinks = getMatchingKinks();
  const partnerDistance = getPartnerDistance();
  const partnerFlag = getPartnerFlag();

  return (
    <div className="fixed inset-0 bg-black flex items-center justify-center">
      {/* Mobile-width container for desktop - with touch handlers for swipe */}
      <div 
        ref={videoContainerRef}
        className="relative w-full h-full max-w-[430px] mx-auto bg-black overflow-hidden"
        onTouchStart={handleTouchStart}
        onTouchMove={handleTouchMove}
        onTouchEnd={handleTouchEnd}
      >
      {/* Floating Hearts Animation */}
      <FloatingHearts show={showHearts} />
      
      {/* Swipe progress indicator (mobile only) */}
      {isMobile && swipeProgress > 0 && connectionState === 'connected' && (
        <div 
          className="absolute top-0 left-0 h-1 bg-white/50 z-50 transition-all"
          style={{ width: `${swipeProgress}%` }}
        />
      )}
      
      {/* Remote Video */}
      <video
        ref={remoteVideoRef}
        autoPlay
        playsInline
        className={`absolute inset-0 w-full h-full object-cover transition-all duration-500 ${connectionState === 'connecting' ? 'blur-2xl scale-110' : ''}`}
      />
      
      {/* ===== SEARCHING STATE - Semi-transparent overlay over frozen frame ===== */}
      {connectionState === 'searching' && (
        <>
          {/* Semi-transparent overlay with blur over frozen last frame */}
          <div className="absolute inset-0 z-10 bg-black/60 backdrop-blur-xl transition-opacity duration-300">
            {/* Subtle metallic gradient overlay */}
            <div
              className="absolute inset-0"
              style={{
                background: 'radial-gradient(ellipse at center top, rgba(40,40,40,0.3) 0%, transparent 60%)',
              }}
            />
          </div>
          
          {/* Local camera preview - harmonized with connected state (top-3 right-3 w-24 h-32) */}
          <div className="absolute top-3 right-3 z-30 w-24 h-32 rounded-2xl overflow-hidden border-2 border-white/30 shadow-xl bg-black">
            <video
              ref={localVideoRef}
              autoPlay
              playsInline
              muted
              className="w-full h-full object-cover"
              style={{ transform: 'scaleX(-1)' }}
            />
          </div>
          
          {/* Searching indicator - bottom center, minimal */}
          <div className="absolute bottom-8 left-1/2 -translate-x-1/2 z-30">
            <div className="flex items-center gap-2">
              <div className="flex gap-1">
                <div className="w-1.5 h-1.5 bg-white rounded-full animate-bounce" style={{ animationDelay: '0ms' }} />
                <div className="w-1.5 h-1.5 bg-white rounded-full animate-bounce" style={{ animationDelay: '150ms' }} />
                <div className="w-1.5 h-1.5 bg-white rounded-full animate-bounce" style={{ animationDelay: '300ms' }} />
              </div>
              <span className="text-white text-sm font-light tracking-wide">
                Recherche
              </span>
            </div>
          </div>
          
          {/* Cancel button - top left - SAME STYLE as live cam active */}
          <div className="absolute top-3 left-3 z-30">
            <div className="flex items-center gap-1.5 px-1.5 py-1.5 rounded-full bg-black/60 backdrop-blur-md">
              <button 
                onClick={handleEndSession} 
                className="w-7 h-7 rounded-full bg-[#3a3a3a] flex items-center justify-center text-white hover:bg-[#4a4a4a] transition-all"
              >
                <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2.5">
                  <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>
          </div>
        </>
      )}
      
      {/* ===== CONNECTING STATE - Clean chrome/black, no blur ===== */}
      {connectionState === 'connecting' && partner && (
        <div className="absolute inset-0 z-20 bg-[#0a0a0a]">
          {/* Simple centered text */}
          <div className="absolute bottom-8 left-1/2 -translate-x-1/2">
            <div className="bg-black rounded-full px-5 py-2.5 flex items-center gap-2">
              <div className="flex gap-1">
                <div className="w-1.5 h-1.5 bg-white rounded-full animate-bounce" style={{ animationDelay: '0ms' }} />
                <div className="w-1.5 h-1.5 bg-white rounded-full animate-bounce" style={{ animationDelay: '150ms' }} />
                <div className="w-1.5 h-1.5 bg-white rounded-full animate-bounce" style={{ animationDelay: '300ms' }} />
              </div>
              <span className="text-white text-xs">Recherche</span>
            </div>
          </div>
          
          {/* Local preview stays visible - harmonized */}
          <div className="absolute top-3 right-3 z-30 w-24 h-32 rounded-2xl overflow-hidden border-2 border-white/30 shadow-xl bg-black">
            <video
              ref={localVideoRef}
              autoPlay
              playsInline
              muted
              className="w-full h-full object-cover"
              style={{ transform: 'scaleX(-1)' }}
            />
          </div>
        </div>
      )}
      
      {/* ===== ERROR STATE ===== */}
      {error && (
        <div className="absolute inset-0 bg-black/90 flex items-center justify-center z-30">
          <div className="text-center">
            <p className="text-white mb-4">{error}</p>
            <button onClick={handleEndSession} className="px-6 py-3 bg-white/10 text-white rounded-xl">Retour</button>
          </div>
        </div>
      )}
      
      {/* ===== CONNECTED UI ===== */}
      {connectionState === 'connected' && (
        <>
          {/* TOP LEFT - Close & Report in ONE blur pill, SMALLER icons */}
          <div className="absolute top-3 left-3 z-10">
            <div className="flex items-center gap-1.5 px-1.5 py-1.5 rounded-full bg-black/60 backdrop-blur-md">
              {/* Close button - smaller */}
              <button onClick={handleEndSession} className="w-7 h-7 rounded-full bg-[#3a3a3a] flex items-center justify-center text-white hover:bg-[#4a4a4a] transition-all" data-testid="close-btn">
                <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2.5">
                  <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
              
              {/* Report button - WHITE SHIELD with exclamation (PJ4) */}
              <button onClick={() => setShowReportOverlay(true)} className="w-7 h-7 rounded-full bg-[#3a3a3a] flex items-center justify-center text-white hover:bg-[#4a4a4a] transition-all" data-testid="report-btn">
                <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M12 2L4 5v6.09c0 5.05 3.41 9.76 8 10.91 4.59-1.15 8-5.86 8-10.91V5l-8-3zm0 15c-.55 0-1-.45-1-1v-1c0-.55.45-1 1-1s1 .45 1 1v1c0 .55-.45 1-1 1zm1-4h-2V8h2v5z"/>
                </svg>
              </button>
            </div>
          </div>
          
          {/* TOP RIGHT - Local Video Preview */}
          <div className="absolute top-3 right-3 z-20 w-24 h-32 rounded-2xl overflow-hidden border-2 border-white/30 shadow-xl bg-black">
            <video
              ref={localVideoRef}
              autoPlay
              playsInline
              muted
              className="w-full h-full object-cover"
              style={{ transform: 'scaleX(-1)' }}
            />
            {/* Switch Camera - overlaid on local preview */}
            <button
              onClick={switchCamera}
              className="absolute bottom-1 right-1 w-7 h-7 rounded-full bg-black/60 backdrop-blur-sm flex items-center justify-center text-white hover:bg-black/80 transition-all"
              data-testid="switch-camera-btn-local"
            >
              <svg className="w-3.5 h-3.5" viewBox="0 0 1536 1024" fill="none" xmlns="http://www.w3.org/2000/svg">
                <path fill="currentColor" fillOpacity="0.996" d="M 628.00 587.42 Q 628.05 587.43 628.20 587.46 A 0.12 0.12 0 0 0 628.32 587.27 Q 627.48 586.26 626.49 586.18 A 1.96 1.96 0 0 1 625.35 585.70 Q 617.08 578.46 613.49 568.07 Q 611.99 563.73 611.92 558.25 C 611.81 548.29 611.35 542.57 611.34 533.25 Q 611.32 506.21 611.37 443.24 Q 611.38 428.66 615.86 419.29 Q 616.85 417.21 619.94 412.35 A 3.71 3.69 87.9 0 1 620.77 411.45 C 623.41 409.40 625.08 406.70 627.75 404.90 Q 631.70 402.25 636.50 400.23 A 2.33 2.27 33.5 0 1 637.51 400.04 Q 638.43 400.08 641.91 399.49 Q 647.74 398.50 658.26 398.48 Q 673.62 398.45 688.24 398.19 Q 696.15 398.04 701.59 393.24 C 704.22 390.92 705.95 387.38 708.53 384.50 A 5.73 5.61 -8.9 0 0 709.12 383.71 Q 713.26 377.06 716.19 373.90 Q 724.62 364.81 738.00 364.04 Q 740.32 363.90 758.19 363.43 C 771.26 363.09 784.17 363.89 798.45 363.40 Q 799.82 363.36 804.24 364.75 Q 807.48 365.77 809.68 366.98 Q 818.19 371.66 823.91 380.60 Q 824.14 380.95 824.53 381.02 A 0.53 0.52 -85.5 0 1 824.97 381.54 Q 824.99 382.21 825.97 383.23 Q 826.48 383.76 827.06 384.77 Q 830.43 390.60 833.28 393.52 C 836.68 397.00 842.10 398.19 847.57 398.21 Q 872.86 398.30 874.35 398.38 A 46.15 45.66 -46.2 0 0 882.00 398.18 Q 882.67 398.11 888.74 398.46 Q 898.48 399.03 907.16 404.88 Q 918.77 412.69 922.50 425.59 Q 923.93 430.51 923.88 443.06 Q 923.57 524.06 923.91 552.03 Q 924.02 561.28 921.19 569.12 C 916.67 581.65 906.07 590.59 893.41 594.13 Q 887.68 595.73 872.96 595.67 Q 851.21 595.60 650.76 595.63 A 2.13 1.93 47.9 0 1 650.36 595.59 Q 648.97 595.28 644.78 594.85 Q 636.47 594.01 627.90 587.66 A 0.14 0.13 -23.1 0 1 628.00 587.42 Z M 843.07 470.73 L 843.07 471.11 A 0.23 0.23 0 0 1 842.62 471.19 Q 837.94 457.17 827.73 445.01 C 825.69 442.58 823.63 440.88 821.34 438.61 Q 813.54 430.85 803.57 426.18 Q 795.01 422.18 787.78 420.54 Q 774.28 417.48 760.26 418.86 Q 752.54 419.62 747.78 421.36 Q 741.94 423.48 735.19 426.19 Q 729.81 428.36 722.41 434.48 Q 718.78 437.48 718.12 439.43 C 715.90 445.97 723.93 450.98 729.10 446.77 Q 740.85 437.22 755.82 433.46 C 757.65 433.00 759.54 433.12 760.63 432.84 A 6.24 6.10 -48.5 0 1 762.83 432.68 C 766.64 433.10 771.64 432.15 775.71 432.55 Q 792.86 434.26 806.70 445.09 Q 821.36 456.56 828.11 473.63 Q 829.05 476.01 829.63 479.55 A 0.63 0.63 0 0 1 829.03 480.28 Q 822.41 480.54 816.79 479.94 Q 815.38 479.79 809.94 480.37 A 2.24 2.20 26.2 0 0 809.03 480.67 Q 804.84 483.09 807.69 487.40 Q 810.78 492.08 820.27 504.96 A 4.35 3.77 17.7 0 1 820.68 505.63 Q 821.68 507.75 823.03 509.25 C 826.31 512.90 828.80 517.19 832.08 521.39 Q 833.33 523.01 834.21 523.24 C 837.17 524.05 840.05 520.81 841.69 518.44 Q 844.50 514.36 848.34 509.34 Q 855.28 500.27 864.29 487.96 A 2.37 2.35 73.7 0 0 864.62 487.35 Q 865.59 484.82 864.69 482.18 A 2.61 2.61 0 0 0 862.22 480.41 L 846.23 480.41 A 1.57 1.57 0 0 1 844.69 479.13 Q 843.90 475.01 843.82 474.91 A 0.97 0.97 0 0 1 843.68 474.06 Q 844.07 472.79 843.25 470.69 A 0.10 0.09 -56.5 0 0 843.07 470.73 Z M 705.78 456.77 Q 700.31 454.22 696.39 459.29 Q 695.26 460.74 688.92 470.14 Q 683.19 478.64 678.92 483.67 Q 677.48 485.36 675.70 488.45 C 674.21 491.04 673.21 495.27 676.50 496.96 A 2.06 2.04 -26.8 0 0 677.18 497.18 C 679.58 497.51 681.73 497.87 683.90 497.83 Q 690.75 497.73 694.22 497.91 A 1.15 1.14 -89.5 0 1 695.31 499.03 Q 695.39 502.64 696.37 507.02 Q 697.26 510.99 698.96 514.68 Q 704.77 527.25 714.72 537.75 C 717.98 541.20 723.01 545.09 726.32 547.13 Q 741.71 556.65 760.04 558.03 Q 770.53 558.82 777.99 558.14 Q 787.21 557.30 792.69 555.90 Q 803.56 553.14 813.28 547.35 C 818.62 544.17 825.02 538.19 820.53 531.70 A 3.42 3.35 2.8 0 0 819.92 531.04 C 816.48 528.30 814.65 526.63 810.50 528.97 Q 806.04 531.49 803.38 533.16 Q 796.70 537.35 796.38 537.51 Q 784.47 543.48 770.77 543.65 Q 760.29 543.78 749.91 540.67 Q 739.76 537.63 731.18 531.13 Q 729.26 529.68 728.06 528.19 C 725.85 525.46 723.52 522.92 721.03 520.21 Q 714.53 513.14 711.65 503.89 Q 710.95 501.61 711.14 499.26 A 1.69 1.68 -88.6 0 1 712.75 497.72 Q 717.19 497.53 724.02 497.74 Q 727.26 497.84 729.94 497.12 A 2.37 2.37 0 0 0 731.67 495.07 Q 732.04 491.45 730.34 489.38 Q 724.35 482.14 721.49 477.81 Q 715.85 469.28 706.81 457.60 A 2.79 2.73 -5.6 0 0 705.78 456.77 Z"/>
              </svg>
            </button>
          </div>

          {/* Watermark pseudo - bottom left, semi-transparent */}
          <span className="absolute left-3 bottom-20 z-10 text-white/20 text-xs pointer-events-none select-none">
            @{user?.display_name || 'brozr'}
          </span>
          
          {/* RIGHT SIDE - Gift & Like (Gift on top, Heart smaller at bottom) */}
          <div className="absolute right-3 top-44 z-10 flex flex-col gap-4 items-center">
            {/* Gift Button - EXACT icon from PJ1 (on TOP now) */}
            <button
              onClick={() => setShowJuiceOverlay(true)}
              className="w-11 h-11 rounded-full flex items-center justify-center hover:scale-110 transition-all bg-[#1a1a1a] border-2 border-white"
              data-testid="juice-btn"
            >
              <svg className="w-6 h-6" viewBox="0 0 24 24" fill="white">
                <ellipse cx="8.5" cy="5.5" rx="2.5" ry="2" />
                <ellipse cx="15.5" cy="5.5" rx="2.5" ry="2" />
                <rect x="4" y="7" width="16" height="4" />
                <rect x="5" y="11" width="14" height="10" />
                <rect x="11" y="7" width="2" height="14" fill="#1a1a1a" />
              </svg>
            </button>
            
            {/* Like Button - SMALLER RED HEART (at BOTTOM now) */}
            <div className="relative flex flex-col items-center">
              <button
                onClick={handleLike}
                disabled={liked}
                className="transition-all hover:scale-110 active:scale-95"
                data-testid="like-btn"
              >
                {/* Smaller Red Heart */}
                <svg className="w-8 h-8" viewBox="0 0 24 24" fill={liked ? '#ffffff' : '#ff0033'}>
                  <path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z"/>
                </svg>
              </button>
              {/* Count BELOW the heart - smaller font */}
              <span 
                key={`like-count-${likesReceived}`}
                className={`text-white text-[10px] font-medium mt-0.5 ${likesReceived > 0 ? 'animate-like-pop' : ''}`}
              >
                {likesReceived >= 1000 ? (likesReceived % 1000 === 0 ? `${Math.floor(likesReceived / 1000)}k` : `${(likesReceived / 1000).toFixed(1)}k`) : likesReceived}
              </span>
              <style>{`
                @keyframes like-pop {
                  0% { transform: scale(1); }
                  50% { transform: scale(1.3); }
                  100% { transform: scale(1); }
                }
                .animate-like-pop { animation: like-pop 0.4s ease-out; }
              `}</style>
            </div>
          </div>
          
          {/* NEXT BUTTON - ALIGNED with profile photo row */}
          <div className="absolute bottom-[85px] right-3 z-10 flex flex-col items-center gap-1">
            <button
              onClick={handleNext}
              className="w-14 h-14 rounded-full bg-white flex items-center justify-center shadow-xl hover:bg-gray-100 active:scale-95 transition-all"
              data-testid="next-btn"
            >
              <svg className="w-6 h-6 text-black" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2.5">
                <path strokeLinecap="round" strokeLinejoin="round" d="M13 7l5 5m0 0l-5 5m5-5H6" />
              </svg>
            </button>
            <span className="text-white text-xs font-medium drop-shadow-lg">Next</span>
          </div>
          
          {/* BOTTOM AREA - Partner Info with Follow button AFTER pseudo */}
          <div className="absolute bottom-16 left-3 right-3 z-10">
            {/* Main row: Photo + Info (with Follow inline) */}
            <div className="flex items-start gap-2">
              {/* Profile Photo */}
              <div className="w-10 h-10 rounded-full border-2 border-white overflow-hidden flex-shrink-0 bg-[#2a2a2a]">
                {partner?.profile_photo ? (
                  <img src={partner.profile_photo} alt="" className="w-full h-full object-cover" />
                ) : (
                  <div className="w-full h-full flex items-center justify-center">
                    <svg className="w-5 h-5 text-white/50" viewBox="0 0 24 24" fill="currentColor">
                      <circle cx="12" cy="8" r="4" />
                      <path d="M12 14c-6 0-8 3-8 6v1h16v-1c0-3-2-6-8-6z" />
                    </svg>
                  </div>
                )}
              </div>
              
              {/* Info block */}
              <div className="flex flex-col gap-0.5 min-w-0">
                {/* Row 1: Name + Age + yo + FOLLOW (same line) */}
                <div className="flex items-center gap-1.5">
                  <span className="text-white font-bold text-sm drop-shadow-lg max-w-[70px] truncate">{partner?.display_name || 'Bro'}</span>
                  <span className="text-white/80 text-sm drop-shadow-lg">{partner?.age ? `${partner.age} yo` : ''}</span>
                  {/* Follow button - SUIVI: black chrome + white text / SUIVRE: white + black text */}
                  <button
                    onClick={handleFollowButtonClick}
                    disabled={followStatus === 'pending'}
                    className={`px-3 py-1 rounded-full font-bold text-xs flex items-center gap-0.5 transition-all ml-2 ${
                      followStatus === 'accepted' 
                        ? 'bg-[#1a1a1a] text-white border border-white/20' 
                        : followStatus === 'pending'
                        ? 'bg-white/30 text-white/70'
                        : 'bg-white text-black hover:bg-gray-100 active:scale-95'
                    }`}
                    data-testid="follow-btn"
                  >
                    {followStatus === 'accepted' ? '✓ Suivi' : followStatus === 'pending' ? '...' : '+ Suivre'}
                  </button>
                </div>
                {/* Row 2: Country + Distance */}
                <div className="flex items-center gap-1.5 text-white/70 text-xs drop-shadow-lg">
                  <span>{partnerFlag}</span>
                  {partnerDistance !== null && <span>{partnerDistance} km</span>}
                </div>
              </div>
            </div>
            
            {/* Kinks Row - ALIGNED LEFT */}
            <div className="flex items-center gap-1 mt-2 ml-0">
              {renderPartnerKinks()}
            </div>
          </div>
          
          {/* BOTTOM FOOTER BAR - THIN footer */}
          <div className="absolute bottom-0 left-0 right-0 z-10">
            <div className="bg-black rounded-t-[16px] px-4 py-2">
              <div className="flex items-center justify-center gap-8">
                {/* Chat */}
                <button
                  onClick={() => setShowChatInput(!showChatInput)}
                  className={`w-9 h-9 rounded-full flex items-center justify-center transition-all ${showChatInput ? 'bg-white text-black' : 'bg-[#333] text-white hover:bg-[#444]'}`}
                  data-testid="chat-btn"
                >
                  <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                    <path d="M21 15a2 2 0 01-2 2H7l-4 4V5a2 2 0 012-2h14a2 2 0 012 2v10z" strokeLinecap="round" strokeLinejoin="round"/>
                  </svg>
                </button>
                
                {/* Filters - with badge showing active kinks count */}
                <button
                  onClick={() => setShowFiltersOverlay(true)}
                  className="relative w-9 h-9 rounded-full bg-[#333] flex items-center justify-center text-white hover:bg-[#444] transition-all"
                  data-testid="filters-btn"
                >
                  <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                    <line x1="4" y1="21" x2="4" y2="14"/>
                    <line x1="4" y1="10" x2="4" y2="3"/>
                    <line x1="12" y1="21" x2="12" y2="12"/>
                    <line x1="12" y1="8" x2="12" y2="3"/>
                    <line x1="20" y1="21" x2="20" y2="16"/>
                    <line x1="20" y1="12" x2="20" y2="3"/>
                    <circle cx="4" cy="12" r="2" fill="currentColor"/>
                    <circle cx="12" cy="10" r="2" fill="currentColor"/>
                    <circle cx="20" cy="14" r="2" fill="currentColor"/>
                  </svg>
                  {/* Badge showing active filters count */}
                  {getActiveFiltersCount() > 0 && (
                    <div className="absolute -top-1 -right-1 min-w-[16px] h-[16px] bg-white text-black text-[10px] font-bold rounded-full flex items-center justify-center">
                      {getActiveFiltersCount()}
                    </div>
                  )}
                </button>
                
                {/* Camera Switch */}
                <button
                  onClick={switchCamera}
                  className="w-9 h-9 rounded-full bg-[#333] flex items-center justify-center text-white hover:bg-[#444] transition-all"
                  data-testid="switch-camera-btn"
                >
                  <svg className="w-4 h-4" viewBox="0 0 24 24" fill="currentColor">
                    <path d="M17.65 6.35C16.2 4.9 14.21 4 12 4C7.58 4 4.01 7.58 4.01 12C4.01 16.42 7.58 20 12 20C15.73 20 18.84 17.45 19.73 14H17.65C16.83 16.33 14.61 18 12 18C8.69 18 6 15.31 6 12C6 8.69 8.69 6 12 6C13.66 6 15.14 6.69 16.22 7.78L13 11H20V4L17.65 6.35Z"/>
                  </svg>
                </button>

                {/* Notification Bell */}
                <button
                  onClick={() => {
                    setShowNotifications(!showNotifications);
                    if (!showNotifications) {
                      fetchNotifications();
                      if (unreadCount > 0) markNotificationsRead();
                    }
                  }}
                  className="w-9 h-9 rounded-full bg-[#333] flex items-center justify-center text-white hover:bg-[#444] transition-all relative"
                  data-testid="notifications-bell-btn"
                >
                  <svg className="w-4 h-4" viewBox="0 0 24 24" fill="currentColor">
                    <path d="M12 22c1.1 0 2-.9 2-2h-4c0 1.1.9 2 2 2zm6-6v-5c0-3.07-1.63-5.64-4.5-6.32V4c0-.83-.67-1.5-1.5-1.5s-1.5.67-1.5 1.5v.68C7.64 5.36 6 7.92 6 11v5l-2 2v1h16v-1l-2-2zm-2 1H8v-6c0-2.48 1.51-4.5 4-4.5s4 2.02 4 4.5v6z"/>
                  </svg>
                  {unreadCount > 0 && (
                    <div className="absolute -top-0.5 -right-0.5 min-w-[16px] h-[16px] bg-red-500 rounded-full flex items-center justify-center">
                      <span className="text-white text-[9px] font-bold">{unreadCount > 9 ? '9+' : unreadCount}</span>
                    </div>
                  )}
                </button>
              </div>
            </div>
          </div>
        </>
      )}
      
      {/* Notifications Overlay */}
      {showNotifications && (
        <div className="absolute inset-x-0 bottom-0 bg-black/95 rounded-t-3xl border-t border-white/10 z-50 flex flex-col max-h-[50vh]">
          <div className="p-4 border-b border-white/10 flex justify-between items-center flex-shrink-0">
            <h3 className="text-white font-bold text-lg">Notifications</h3>
            <button onClick={() => setShowNotifications(false)} className="w-8 h-8 flex items-center justify-center text-white/50 hover:text-white text-xl">✕</button>
          </div>
          <div className="overflow-y-auto flex-1 p-4">
            {notifications.length === 0 ? (
              <p className="text-white/50 text-center py-4">Aucune notification</p>
            ) : (
              <div className="space-y-3">
                {notifications
                  .filter(notif => notif.type !== 'livecam_response') // Hide livecam response notifications
                  .map((notif) => (
                  <div key={notif.id} className={`p-3 rounded-xl ${notif.read ? 'bg-white/5' : 'bg-white/10'}`}>
                    <p className="text-white text-sm">
                      {notif.type === 'follow_request' && (
                        <>👤 <span className="font-bold">@{notif.from_user?.display_name || notif.from_user_name || 'Bro'}</span> veut te suivre</>
                      )}
                      {notif.type === 'follow_accepted' && (
                        <>✅ <span className="font-bold">@{notif.from_user?.display_name || notif.from_user_name || 'Bro'}</span> a accepté ta demande</>
                      )}
                      {notif.type === 'like' && (
                        <>❤️ <span className="font-bold">@{notif.from_user?.display_name || notif.from_user_name || 'Bro'}</span> t&apos;a liké</>
                      )}
                      {notif.type === 'new_message' && (
                        <>💬 <span className="font-bold">@{notif.from_user?.display_name || notif.from_user_name || 'Bro'}</span> t&apos;a envoyé un message</>
                      )}
                      {notif.type === 'livecam_request' && (
                        <>📹 <span className="font-bold">@{notif.from_user?.display_name || notif.from_user_name || 'Bro'}</span> te demande un Live Cam</>
                      )}
                      {/* Fallback for unknown types - show message_preview if available */}
                      {!['follow_request', 'follow_accepted', 'like', 'new_message', 'livecam_request', 'livecam_response'].includes(notif.type) && (
                        notif.message_preview 
                          ? <>{notif.message_preview}</>
                          : <>🔔 <span className="font-bold">@{notif.from_user?.display_name || notif.from_user_name || 'Bro'}</span> - Notification</>
                      )}
                    </p>
                    <p className="text-white/40 text-xs mt-1">
                      {new Date(notif.created_at).toLocaleString('fr-FR', { day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit' })}
                    </p>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      )}
      
      {/* ===== OVERLAYS ===== */}
      
      {/* Incoming Follow Request Popup - Same style as other overlays */}
      {showIncomingFollowRequest && followRequester && (
        <div className="absolute inset-x-0 bottom-0 bg-black/95 rounded-t-3xl border-t border-white/10 z-50 flex flex-col max-h-[50vh]">
          <div className="p-4 border-b border-white/10 flex justify-between items-center flex-shrink-0">
            <h3 className="text-white font-bold text-lg">Demande de suivi</h3>
            <button onClick={() => handleRespondFollowRequest(false)} className="w-8 h-8 flex items-center justify-center text-white/50 hover:text-white text-xl">✕</button>
          </div>
          <div className="p-6 flex flex-col items-center">
            <div className="w-16 h-16 rounded-full border-2 border-white overflow-hidden mb-3 bg-[#2a2a2a]">
              {followRequester.photo ? (
                <img src={followRequester.photo} alt="" className="w-full h-full object-cover" />
              ) : (
                <div className="w-full h-full flex items-center justify-center">
                  <svg className="w-8 h-8 text-white/50" viewBox="0 0 24 24" fill="currentColor">
                    <circle cx="12" cy="8" r="4" />
                    <path d="M12 14c-6 0-8 3-8 6v1h16v-1c0-3-2-6-8-6z" />
                  </svg>
                </div>
              )}
            </div>
            <h4 className="text-white font-bold text-lg">{followRequester.name}</h4>
            <p className="text-white/60 text-sm mb-6">veut te suivre</p>
            <div className="flex gap-3 w-full max-w-xs">
              <button
                onClick={() => handleRespondFollowRequest(true)}
                className="flex-1 py-3 bg-white text-black rounded-xl font-bold hover:bg-gray-100 transition-all active:scale-[0.98]"
                data-testid="follow-accept-btn"
              >
                Accepter
              </button>
              <button
                onClick={() => handleRespondFollowRequest(false)}
                className="flex-1 py-3 bg-white/10 text-white rounded-xl font-medium hover:bg-white/20 transition-all"
                data-testid="follow-refuse-btn"
              >
                Refuser
              </button>
            </div>
          </div>
        </div>
      )}
      
      {/* Unfollow Confirmation Popup */}
      {showUnfollowConfirm && (
        <div className="absolute inset-0 bg-black/70 z-50 flex items-center justify-center" onClick={() => setShowUnfollowConfirm(false)}>
          <div className="bg-[#1a1a1a] rounded-2xl p-6 mx-6 max-w-xs w-full border border-white/10" onClick={(e) => e.stopPropagation()}>
            <h3 className="text-white font-bold text-lg text-center mb-2">Ne plus suivre ?</h3>
            <p className="text-white/60 text-sm text-center mb-6">
              Tu ne suivras plus {partner?.display_name || 'cet utilisateur'}
            </p>
            <div className="flex gap-3">
              <button
                onClick={() => setShowUnfollowConfirm(false)}
                className="flex-1 py-2.5 bg-white/10 text-white rounded-xl font-medium hover:bg-white/20 transition-all"
              >
                Annuler
              </button>
              <button
                onClick={handleUnfollow}
                className="flex-1 py-2.5 bg-red-500 text-white rounded-xl font-bold hover:bg-red-600 transition-all"
                data-testid="unfollow-confirm-btn"
              >
                Ne plus suivre
              </button>
            </div>
          </div>
        </div>
      )}
      
      {/* Kinks Overlay - Same style as Filters */}
      {showKinksOverlay && partner && (
        <div className="absolute inset-x-0 bottom-0 bg-black/95 rounded-t-3xl border-t border-white/10 z-40 flex flex-col max-h-[60vh]">
          <div className="p-4 border-b border-white/10 flex justify-between items-center flex-shrink-0">
            <h3 className="text-white font-bold text-lg">Kinks de {partner.display_name}</h3>
            <button onClick={() => setShowKinksOverlay(false)} className="w-8 h-8 flex items-center justify-center text-white/50 hover:text-white text-xl">✕</button>
          </div>
          <div className="p-4 overflow-y-auto flex-1">
            <div className="flex flex-wrap gap-2">
              {renderAllPartnerKinks()}
            </div>
            {matchingKinks.length > 0 && <p className="text-white text-sm mt-4 font-medium">{matchingKinks.length} kink{matchingKinks.length > 1 ? 's' : ''} recherché{matchingKinks.length > 1 ? 's' : ''} !</p>}
          </div>
        </div>
      )}
      
      {/* Juice Overlay */}
      {showJuiceOverlay && (
        <div className="absolute inset-x-0 bottom-0 bg-black/95 rounded-t-3xl border-t border-white/10 z-40 flex flex-col">
          <div className="p-4 border-b border-white/10 flex justify-between items-center flex-shrink-0">
            <h3 className="text-white font-bold text-lg">Juice</h3>
            <button onClick={() => setShowJuiceOverlay(false)} className="w-8 h-8 flex items-center justify-center text-white/50 hover:text-white text-xl">✕</button>
          </div>
          <div className="p-6 text-center">
            <div className="text-5xl mb-4">🎁</div>
            <h4 className="text-white font-bold text-lg mb-2">Bientôt disponible</h4>
            <p className="text-white/60 text-sm">Envoie du Juice à ton bro !</p>
          </div>
        </div>
      )}
      
      {/* Report Overlay - 2-step flow */}
      {showReportOverlay && (
        <div className="absolute inset-x-0 bottom-0 bg-black/95 rounded-t-3xl border-t border-white/10 z-40 flex flex-col max-h-[50vh]">
          <div className="p-4 border-b border-white/10 flex justify-between items-center flex-shrink-0">
            <h3 className="text-white font-bold text-lg">
              {reportStep === 1 ? 'Signaler' : 'Ajouter un commentaire'}
            </h3>
            <button onClick={() => { setShowReportOverlay(false); setReportStep(1); setReportReason(''); setReportComment(''); }} className="w-8 h-8 flex items-center justify-center text-white/50 hover:text-white text-xl">✕</button>
          </div>
          <div className="p-4 overflow-y-auto flex-1">
            {reportStep === 1 ? (
              <div className="flex flex-col gap-2">
                {['Activité illégale ou mineur', 'Harcèlement ou menace', 'Spam ou arnaque', 'Partage sans consentement', 'Autre'].map((reason) => (
                  <button
                    key={reason}
                    onClick={() => { setReportReason(reason); setReportStep(2); }}
                    className="w-full py-3 bg-white/10 hover:bg-white/20 text-white rounded-xl text-sm text-left px-4 transition-all"
                  >
                    {reason}
                  </button>
                ))}
              </div>
            ) : (
              <div className="space-y-4">
                <p className="text-white/60 text-sm">Raison : <span className="text-white font-medium">{reportReason}</span></p>
                <textarea
                  value={reportComment}
                  onChange={(e) => setReportComment(e.target.value.slice(0, 150))}
                  placeholder="Ajoute un commentaire (optionnel)..."
                  maxLength={150}
                  className="w-full bg-white/5 border border-white/10 focus:border-white/30 focus:outline-none p-3 rounded-xl text-white min-h-[80px] resize-none placeholder:text-white/30 text-sm"
                  style={{ fontSize: '16px' }}
                />
                <p className="text-white/30 text-xs text-right">{reportComment.length}/150</p>
                <button
                  onClick={() => {
                    setShowReportOverlay(false);
                    setReportStep(1);
                    setReportReason('');
                    setReportComment('');
                    setShowReportConfirm(true);
                    setTimeout(() => setShowReportConfirm(false), 3000);
                  }}
                  className="w-full py-3 bg-white text-black rounded-xl font-bold text-sm transition-all active:scale-[0.98]"
                >
                  Envoyer le signalement
                </button>
              </div>
            )}
          </div>
        </div>
      )}
      
      {/* Report Confirmation Message */}
      {showReportConfirm && (
        <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 animate-fade-in">
          <div className="bg-black/90 backdrop-blur-xl border border-white/20 rounded-2xl px-8 py-6 text-center shadow-2xl">
            <div className="w-14 h-14 mx-auto mb-4 rounded-full bg-white/10 flex items-center justify-center">
              <svg className="w-7 h-7 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
              </svg>
            </div>
            <h3 className="text-white font-bold text-lg mb-2">Signalement envoyé</h3>
            <p className="text-white/60 text-sm">Merci de contribuer à la sécurité de Brozr.</p>
          </div>
        </div>
      )}
      
      {/* Chat Input Bar - positioned CLEARLY ABOVE the bottom left info block */}
      {/* Chat Input - Fixed at bottom, styled for mobile and desktop */}
      {showChatInput && connectionState === 'connected' && (
        <>
          {/* Backdrop to close chat on click anywhere */}
          <div 
            className="fixed inset-0 z-40" 
            onClick={() => setShowChatInput(false)}
          />
          {/* Chat input bar - contained width on desktop, full on mobile */}
          <div 
            className="fixed left-0 right-0 bottom-0 z-50 bg-gray-100 border-t border-gray-300 px-3 py-2 flex justify-center"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex gap-2 items-center bg-white rounded-full px-4 py-2 shadow-sm w-full max-w-md">
              <input
                type="text"
                value={chatMessage}
                onChange={(e) => setChatMessage(e.target.value)}
                onKeyDown={(e) => { if (e.key === 'Enter') { e.preventDefault(); sendChatMessage(); } }}
                placeholder="Saisis ton message ici"
                className="flex-1 bg-transparent text-gray-800 placeholder-gray-400 focus:outline-none min-w-0"
                style={{ fontSize: '16px' }}
                data-testid="chat-input"
                autoFocus
                autoComplete="off"
                autoCorrect="off"
              />
              <button
                type="button"
                onClick={() => setShowChatInput(false)}
                className="w-8 h-8 flex items-center justify-center text-gray-400 hover:text-gray-600 transition-colors flex-shrink-0"
                data-testid="chat-close-btn"
              >
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2.5">
                  <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
              <button
                type="button"
                onClick={sendChatMessage}
                className="w-10 h-10 flex items-center justify-center bg-blue-500 text-white rounded-full hover:bg-blue-600 active:bg-blue-700 transition-colors flex-shrink-0"
                data-testid="chat-send-btn"
              >
                <svg className="w-5 h-5" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z"/>
                </svg>
              </button>
            </div>
          </div>
        </>
      )}
      
      {/* Floating Chat Messages - ABOVE partner info (higher position) */}
      {connectionState === 'connected' && chatMessages.length > 0 && (
        <div 
          className="absolute left-3 z-20 space-y-1 pointer-events-none" 
          style={{ bottom: showChatInput ? '220px' : '160px', maxWidth: '55%' }}
        >
          {chatMessages.slice(-3).map((msg) => (
            <div 
              key={msg.id} 
              className={`px-2.5 py-1 rounded-full backdrop-blur-sm ${
                msg.fromPartner 
                  ? 'bg-black/40 text-white/90' 
                  : 'bg-white/90 text-gray-800'
              }`}
              style={{ animation: 'msg-appear 0.3s ease-out', fontSize: '11px' }}
            >
              {msg.message}
            </div>
          ))}
          <style>{`
            @keyframes msg-appear { 
              0% { opacity: 0; transform: translateY(5px); } 
              100% { opacity: 1; transform: translateY(0); }
            }
          `}</style>
        </div>
      )}
      
      {/* FULL FILTERS OVERLAY - Same as LivePrematch */}
      {/* Filters Overlay - Same white theme as LivePrematch */}
      {showFiltersOverlay && (
        <div className="absolute inset-x-0 bottom-0 bg-black/95 rounded-t-3xl border-t border-white/10 z-40 flex flex-col max-h-[60vh]">
          <div className="p-4 border-b border-white/10 flex justify-between items-center flex-shrink-0">
            <span className="text-white font-bold text-lg">Looking for</span>
            <div className="flex items-center gap-3">
              <button onClick={() => { setTempAgeMin(18); setTempAgeMax(60); setTempDistance(400); setTempKinks([]); }} className="text-white/70 text-sm font-medium hover:text-white">Réinitialiser</button>
              <button onClick={() => setShowFiltersOverlay(false)} className="w-8 h-8 flex items-center justify-center text-white/50 hover:text-white text-xl">✕</button>
            </div>
          </div>
          
          <div className="p-4 overflow-y-auto flex-1">
            {/* Age - Dual Range Slider */}
            <div className="mb-6">
              <div className="flex justify-between mb-3">
                <span className="text-white font-medium">Âge</span>
                <span className="text-white font-bold">{tempAgeMin} - {tempAgeMax === 60 ? '60+' : tempAgeMax} ans</span>
              </div>
              <DualRangeSlider min={18} max={60} minVal={tempAgeMin} maxVal={tempAgeMax} onMinChange={setTempAgeMin} onMaxChange={setTempAgeMax} />
              <div className="flex justify-between text-white/30 text-xs mt-2">
                <span>18 ans</span>
                <span>60+ ans</span>
              </div>
            </div>

            {/* Distance - Same style as LivePrematch */}
            <div className="mb-5">
              <div className="flex justify-between mb-3">
                <span className="text-white font-medium">Distance</span>
                <span className="text-white font-bold">{tempDistance === 400 ? '400+' : tempDistance} km</span>
              </div>
              <div className="relative h-6 flex items-center">
                <div className="absolute w-full h-2 bg-white/10 rounded-full"></div>
                <div className="absolute h-2 bg-white rounded-full" style={{ left: '0%', right: (100 - ((tempDistance - 1) / 399) * 100) + '%' }}></div>
                <input
                  type="range"
                  min="1"
                  max="400"
                  value={tempDistance}
                  onChange={e => setTempDistance(parseInt(e.target.value))}
                  className="absolute w-full h-2 appearance-none bg-transparent cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-5 [&::-webkit-slider-thumb]:h-5 [&::-webkit-slider-thumb]:bg-white [&::-webkit-slider-thumb]:border-2 [&::-webkit-slider-thumb]:border-white [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:cursor-pointer [&::-webkit-slider-thumb]:shadow-lg [&::-moz-range-thumb]:appearance-none [&::-moz-range-thumb]:w-5 [&::-moz-range-thumb]:h-5 [&::-moz-range-thumb]:bg-white [&::-moz-range-thumb]:border-2 [&::-moz-range-thumb]:border-white [&::-moz-range-thumb]:rounded-full [&::-moz-range-thumb]:cursor-pointer"
                />
              </div>
              <div className="flex justify-between text-white/30 text-xs mt-2">
                <span>1 km</span>
                <span>400+ km</span>
              </div>
            </div>

            {/* Kinks */}
            <div className="mb-2">
              <div className="flex justify-between mb-2">
                <span className="text-white font-medium">Kinks & Rôles</span>
                <span className="text-white font-bold">{tempKinks.length} sélectionnés</span>
              </div>
              {renderKinkCategories()}
            </div>
          </div>

          {/* Save CTA - White button */}
          <div className="p-4 border-t border-white/10 flex-shrink-0 flex justify-center">
            <button onClick={saveFilters} className="py-3 px-10 bg-white text-black rounded-xl font-bold text-base transition-all active:scale-[0.98]">
              Enregistrer
            </button>
          </div>
        </div>
      )}
      </div>
    </div>
  );
};

export default VideoCall;
