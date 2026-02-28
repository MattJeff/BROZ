import React, { useState, useEffect, useRef, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '@/contexts/AuthContext';
import { toast } from 'sonner';
import { KINKS_FLAT } from '@/utils/kinks';

const API_URL = process.env.REACT_APP_BACKEND_URL;

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

const kinkCategories = KINKS_FLAT;

function KinkChip({ label, selected, onToggle }) {
  const cls = selected
    ? "px-3 py-1.5 rounded-full text-sm bg-white text-black transition-all"
    : "px-3 py-1.5 rounded-full text-sm bg-white/5 text-white/70 hover:bg-white/10 border border-white/10 transition-all";
  return <button type="button" onClick={onToggle} className={cls}>{label}</button>;
}

function DualRangeSlider({ min, max, minVal, maxVal, onMinChange, onMaxChange }) {
  const range = max - min;
  const minPercent = ((minVal - min) / range) * 100;
  const maxPercent = ((maxVal - min) / range) * 100;

  return (
    <div className="relative h-6 flex items-center">
      {/* Track background */}
      <div className="absolute w-full h-2 bg-white/10 rounded-full"></div>
      
      {/* Active track */}
      <div 
        className="absolute h-2 bg-white rounded-full"
        style={{ left: minPercent + '%', right: (100 - maxPercent) + '%' }}
      ></div>
      
      {/* Min slider */}
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
      
      {/* Max slider */}
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

export const LivePrematch = () => {
  const navigate = useNavigate();
  const { user } = useAuth();
  const videoRef = useRef(null);
  const streamRef = useRef(null);
  
  const [facingMode, setFacingMode] = useState('user');
  const [cameraReady, setCameraReady] = useState(false);
  const [cameraError, setCameraError] = useState(null);
  const [isSearching, setIsSearching] = useState(false);
  const [isMobile, setIsMobile] = useState(false);
  const [showGoLiveHint, setShowGoLiveHint] = useState(true);
  const [showProfileMenu, setShowProfileMenu] = useState(false);
  
  // Detect if device is touch-capable (mobile) on mount
  useEffect(() => {
    const checkMobile = () => {
      const mobile = ('ontouchstart' in window) || 
        (navigator.maxTouchPoints > 0) ||
        window.matchMedia('(pointer: coarse)').matches ||
        window.matchMedia('(max-width: 768px)').matches ||
        /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(navigator.userAgent);
      setIsMobile(mobile);
    };
    checkMobile();
    window.addEventListener('resize', checkMobile);
    return () => window.removeEventListener('resize', checkMobile);
  }, []);
  
  const [showCountry, setShowCountry] = useState(false);
  const [showFilters, setShowFilters] = useState(false);
  const [showNotif, setShowNotif] = useState(false);
  // Track unread notification count from server
  const [unreadCount, setUnreadCount] = useState(0);
  const [notifications, setNotifications] = useState([]);
  const [messageUnreadCount, setMessageUnreadCount] = useState(0);
  
  // Fetch unread count from server
  const fetchUnreadCount = useCallback(async () => {
    try {
      const token = localStorage.getItem('brozr_token');
      if (!token) return;
      
      const res = await fetch(`${API_URL}/api/notifications/unread-count`, {
        headers: { 'Authorization': `Bearer ${token}` }
      });
      if (res.ok) {
        const data = await res.json();
        setUnreadCount(data.count);
      }
    } catch (err) {
      console.error('Failed to fetch unread count:', err);
    }
  }, []);

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
  
  // Fetch notifications list
  const fetchNotifications = useCallback(async () => {
    try {
      const token = localStorage.getItem('brozr_token');
      if (!token) return;
      
      const res = await fetch(`${API_URL}/api/notifications`, {
        headers: { 'Authorization': `Bearer ${token}` }
      });
      if (res.ok) {
        const data = await res.json();
        setNotifications(data);
      }
    } catch (err) {
      console.error('Failed to fetch notifications:', err);
    }
  }, []);
  
  // Mark all notifications as read
  const markAllAsRead = useCallback(async () => {
    try {
      const token = localStorage.getItem('brozr_token');
      if (!token) return;
      
      await fetch(`${API_URL}/api/notifications/mark-all-read`, {
        method: 'POST',
        headers: { 'Authorization': `Bearer ${token}` }
      });
      setUnreadCount(0);
    } catch (err) {
      console.error('Failed to mark notifications as read:', err);
    }
  }, []);
  
  // Fetch unread count on mount and periodically
  useEffect(() => {
    fetchUnreadCount();
    fetchMessageUnreadCount();
    // Refresh counts every 30 seconds
    const interval = setInterval(() => {
      fetchUnreadCount();
      fetchMessageUnreadCount();
    }, 30000);
    return () => clearInterval(interval);
  }, [fetchUnreadCount, fetchMessageUnreadCount]);
  
  // Handle notification bell click
  const handleNotifClick = async () => {
    const wasOpen = showNotif;
    setShowNotif(!showNotif);
    
    if (!wasOpen) {
      // Opening: fetch notifications and mark as read
      await fetchNotifications();
      if (unreadCount > 0) {
        await markAllAsRead();
      }
    }
  };
  
  const [activeFilters, setActiveFilters] = useState({
    country: null,
    ageMin: 18,
    ageMax: 60,
    distance: 400
  });
  
  const [tempAgeMin, setTempAgeMin] = useState(18);
  const [tempAgeMax, setTempAgeMax] = useState(60);
  const [tempDistance, setTempDistance] = useState(400);
  const [tempKinks, setTempKinks] = useState([]);
  const [activeKinks, setActiveKinks] = useState([]);
  const [countryLoading, setCountryLoading] = useState(true);

  // Calculate total active filters count (age, distance, kinks)
  const getActiveFiltersCount = () => {
    let count = activeKinks.length;
    // Count age if modified from defaults (18-60)
    if (activeFilters.ageMin !== 18 || activeFilters.ageMax !== 60) {
      count++;
    }
    // Count distance if modified from default (400)
    if (activeFilters.distance !== 400) {
      count++;
    }
    return count;
  };

  // Load saved filters from localStorage on mount
  useEffect(() => {
    try {
      const savedFilters = localStorage.getItem('brozr_match_filters');
      if (savedFilters) {
        const filters = JSON.parse(savedFilters);
        if (filters.minAge || filters.ageMin) {
          const minAge = filters.minAge || filters.ageMin;
          setTempAgeMin(minAge);
          setActiveFilters(prev => ({ ...prev, ageMin: minAge }));
        }
        if (filters.maxAge || filters.ageMax) {
          const maxAge = filters.maxAge || filters.ageMax;
          setTempAgeMax(maxAge);
          setActiveFilters(prev => ({ ...prev, ageMax: maxAge }));
        }
        if (filters.distance) {
          setTempDistance(filters.distance);
          setActiveFilters(prev => ({ ...prev, distance: filters.distance }));
        }
        if (filters.kinks) {
          setTempKinks(filters.kinks);
          setActiveKinks(filters.kinks);
        }
        if (filters.country) {
          setActiveFilters(prev => ({ ...prev, country: filters.country }));
        }
      }
    } catch (e) {
      console.error('Error loading saved filters:', e);
    }
  }, []);

  const isProfileComplete = user?.profile_photo && user?.display_name && user?.kinks?.length > 0;
  const currentCountry = activeFilters.country === 'ALL' ? { code: 'ALL', name: 'Monde', flag: '🌍' } : (countryList.find(c => c.code === activeFilters.country) || { code: 'ALL', name: 'Monde', flag: '🌍' });

  // Detect user's country on mount (from profile first, then IP fallback)
  useEffect(() => {
    const detectCountry = async () => {
      try {
        // First check if there's a saved country in localStorage
        const savedFilters = localStorage.getItem('brozr_match_filters');
        if (savedFilters) {
          const filters = JSON.parse(savedFilters);
          if (filters.country && filters.country !== 'ALL') {
            setActiveFilters(prev => ({ ...prev, country: filters.country }));
            setCountryLoading(false);
            return;
          }
        }

        // Use user profile country if available
        if (user?.country && user.country !== '') {
          const profileCountry = countryList.find(c => c.code === user.country);
          if (profileCountry) {
            setActiveFilters(prev => ({ ...prev, country: user.country }));
            setCountryLoading(false);
            return;
          }
        }

        // Fallback: detect from IP
        const res = await fetch('https://ipapi.co/json/');
        const data = await res.json();
        const countryCode = data.country_code;
        // Map GB to UK for consistency
        const mappedCode = countryCode === 'GB' ? 'UK' : countryCode;
        // Check if detected country is in our list
        const foundCountry = countryList.find(c => c.code === mappedCode);
        if (foundCountry) {
          setActiveFilters(prev => ({ ...prev, country: mappedCode }));
        } else {
          setActiveFilters(prev => ({ ...prev, country: 'ALL' }));
        }
      } catch (err) {
        // Fallback to ALL if detection fails
        setActiveFilters(prev => ({ ...prev, country: 'ALL' }));
      } finally {
        setCountryLoading(false);
      }
    };
    detectCountry();
  }, []);

  // Hide Go Live hint after 60 seconds
  useEffect(() => {
    const timer = setTimeout(() => setShowGoLiveHint(false), 60000);
    return () => clearTimeout(timer);
  }, []);

  useEffect(() => {
    startCamera();
    return () => stopCamera();
  }, [facingMode]);

  const startCamera = async () => {
    try {
      setCameraError(null);
      if (streamRef.current) {
        streamRef.current.getTracks().forEach(track => track.stop());
      }
      const stream = await navigator.mediaDevices.getUserMedia({
        video: { facingMode, width: { ideal: 1280 }, height: { ideal: 720 } },
        audio: true
      });
      streamRef.current = stream;
      if (videoRef.current) {
        videoRef.current.srcObject = stream;
        setCameraReady(true);
      }
    } catch (err) {
      setCameraError('Autorise ta caméra');
      setCameraReady(false);
    }
  };

  const stopCamera = () => {
    if (streamRef.current) {
      streamRef.current.getTracks().forEach(track => track.stop());
    }
  };

  const switchCamera = () => setFacingMode(f => f === 'user' ? 'environment' : 'user');

  const handleMatch = () => {
    if (!cameraReady) { return; }
    
    // Stop the camera before navigating
    stopCamera();
    
    // Build filters object for matching
    const matchFilters = {
      country: activeFilters.country,
      ageMin: activeFilters.ageMin,
      ageMax: activeFilters.ageMax,
      distance: activeFilters.distance,
      kinks: activeKinks
    };
    
    // Save filters to localStorage for VideoCall to pick up (persisted across sessions)
    localStorage.setItem('brozr_match_filters', JSON.stringify(matchFilters));
    
    // Navigate to safety screen first (mandatory before each live session)
    navigate('/safety');
  };

  const openFilters = () => {
    setTempAgeMin(activeFilters.ageMin);
    setTempAgeMax(activeFilters.ageMax);
    setTempDistance(activeFilters.distance);
    setTempKinks([...activeKinks]);
    setShowFilters(true);
    setShowCountry(false);
  };

  const saveFilters = () => {
    setActiveFilters(prev => ({ ...prev, ageMin: tempAgeMin, ageMax: tempAgeMax, distance: tempDistance }));
    setActiveKinks([...tempKinks]);
    
    // Save to localStorage immediately so filters persist across sessions
    const filtersToSave = {
      country: activeFilters.country,
      minAge: tempAgeMin,
      maxAge: tempAgeMax,
      ageMin: tempAgeMin,
      ageMax: tempAgeMax,
      distance: tempDistance,
      kinks: tempKinks
    };
    localStorage.setItem('brozr_match_filters', JSON.stringify(filtersToSave));
    setShowFilters(false);
  };

  const selectCountry = (code) => {
    setActiveFilters(prev => ({ ...prev, country: code }));
    
    // Also update localStorage with new country
    try {
      const savedFilters = localStorage.getItem('brozr_match_filters');
      const currentFilters = savedFilters ? JSON.parse(savedFilters) : {};
      currentFilters.country = code;
      localStorage.setItem('brozr_match_filters', JSON.stringify(currentFilters));
    } catch (e) {
      console.error('Error saving country to localStorage:', e);
    }
    
    setShowCountry(false);
  };

  const toggleTempKink = (k) => {
    if (tempKinks.includes(k)) {
      setTempKinks(tempKinks.filter(x => x !== k));
    } else if (tempKinks.length < 10) {
      setTempKinks([...tempKinks, k]);
    } else {
      toast.error('Maximum 10 kinks');
    }
  };

  const renderCountryList = () => {
    const items = [];
    // "Tous les pays" option first
    const isAllSelected = activeFilters.country === 'ALL';
    items.push(
      <button key="ALL" onClick={() => selectCountry('ALL')} className={`flex items-center gap-3 p-3 rounded-xl transition-all col-span-2 ${isAllSelected ? 'bg-white text-black' : 'bg-white/5 text-white/80 hover:bg-white/10'}`}>
        <span className="text-2xl">🌍</span>
        <span className="font-medium text-sm">Tous les pays</span>
      </button>
    );
    for (let i = 0; i < countryList.length; i++) {
      const c = countryList[i];
      const isSelected = activeFilters.country === c.code;
      items.push(
        <button key={c.code} onClick={() => selectCountry(c.code)} className={`flex items-center gap-3 p-3 rounded-xl transition-all ${isSelected ? 'bg-white text-black' : 'bg-white/5 text-white/80 hover:bg-white/10'}`}>
          <span className="text-2xl">{c.flag}</span>
          <span className="font-medium text-sm">{c.name}</span>
        </button>
      );
    }
    return items;
  };

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

  return (
    <div className="fixed inset-0 bg-black flex items-center justify-center">
      {/* Mobile-width container for desktop */}
      <div className="relative w-full h-full max-w-[430px] mx-auto bg-black flex flex-col overflow-hidden">
      <div className="flex-1 relative overflow-hidden">
        {/* Video container - full screen with rounded bottom only */}
        <div className="absolute inset-0 overflow-hidden rounded-b-[28px]">
          {cameraError ? (
            <div className="absolute inset-0 flex items-center justify-center bg-gradient-to-b from-gray-900 to-black">
              <div className="text-center p-6">
                <div className="w-20 h-20 mx-auto mb-4 rounded-full bg-white/10 flex items-center justify-center">
                  <svg className="w-10 h-10 text-white/50" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
                  </svg>
                </div>
                <p className="text-white/70 mb-4">{cameraError}</p>
                <button onClick={startCamera} className="px-6 py-3 bg-white text-black rounded-xl font-medium">Autoriser</button>
              </div>
            </div>
          ) : (
            <video ref={videoRef} autoPlay playsInline muted className="absolute inset-0 w-full h-full object-cover" style={{ transform: facingMode === 'user' ? 'scaleX(-1)' : 'none' }} />
          )}
        </div>

        {/* Top Bar - Profile left, Controls right */}
        <div className="absolute top-3 left-3 z-20">
          {/* Profile Photo - Opens dropdown menu */}
          <button onClick={() => setShowProfileMenu(!showProfileMenu)} className="relative">
            <div className="w-9 h-9 rounded-full border-2 border-white overflow-hidden shadow-lg">
              {user?.profile_photo ? (
                <img src={user.profile_photo} alt="" className="w-full h-full object-cover" />
              ) : (
                <div className="w-full h-full bg-gray-800 flex items-center justify-center">
                  <svg className="w-4 h-4 text-white/70" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
                  </svg>
                </div>
              )}
            </div>
            {!isProfileComplete && (
              <div className="absolute -top-0.5 -right-0.5 w-3.5 h-3.5 bg-orange-500 rounded-full flex items-center justify-center">
                <span className="text-white text-[8px] font-bold">!</span>
              </div>
            )}
          </button>

          {/* Profile Dropdown Menu */}
          {showProfileMenu && (
            <div className="absolute top-12 left-0 w-52 bg-black/95 rounded-2xl border border-white/10 z-30">
              <div className="p-2">
                <button
                  onClick={() => { navigate('/profile'); setShowProfileMenu(false); }}
                  className="w-full flex items-center gap-3 p-3 rounded-xl hover:bg-white/10 transition-all"
                >
                  <svg className="w-5 h-5 text-white/70" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                    <path strokeLinecap="round" strokeLinejoin="round" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
                  </svg>
                  <span className="text-white text-sm font-medium">Mon Profil</span>
                </button>
              </div>
            </div>
          )}
        </div>

        {/* Right side controls - ALIGNED with profile photo */}
        <div className="absolute top-3 right-3 bottom-24 flex flex-col items-center justify-between z-20">
          {/* Top: Bell + Camera in blur pill - SMALLER icons */}
          <div className="relative flex flex-col items-center">
            {/* Blur background pill */}
            <div className="absolute -inset-1 bg-black/50 backdrop-blur-md rounded-full"></div>
            
            <div className="relative flex flex-col items-center gap-1 py-1 px-1">
              {/* Notification Bell - Smaller w-7 h-7 */}
              <button onClick={handleNotifClick} className="relative w-7 h-7 flex items-center justify-center">
                <svg className="w-4 h-4 text-white" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                  <path d="M12 2C10.3431 2 9 3.34315 9 5V5.34141C6.66962 6.16508 5 8.38756 5 11V14.1585C5 14.6973 4.78595 15.2141 4.40493 15.5951L3 17H21L19.5951 15.5951C19.2141 15.2141 19 14.6973 19 14.1585V11C19 8.38756 17.3304 6.16508 15 5.34141V5C15 3.34315 13.6569 2 12 2Z" strokeLinecap="round" strokeLinejoin="round"/>
                  <path d="M9 17V18C9 19.6569 10.3431 21 12 21C13.6569 21 15 19.6569 15 18V17" strokeLinecap="round" strokeLinejoin="round"/>
                </svg>
                {unreadCount > 0 && (
                  <div className="absolute -top-1 -right-1 min-w-[12px] h-[12px] bg-red-500 rounded-full flex items-center justify-center px-0.5">
                    <span className="text-white text-[7px] font-bold">{unreadCount > 9 ? '9+' : unreadCount}</span>
                  </div>
                )}
              </button>

              {/* Switch Camera Button - Smaller w-7 h-7 */}
              <button onClick={switchCamera} className="w-7 h-7 flex items-center justify-center">
                <svg className="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="white" strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round">
                  {/* Camera body */}
                  <path d="M2 8.5A1.5 1.5 0 013.5 7H6l1.5-2.5h9L18 7h2.5A1.5 1.5 0 0122 8.5V18a1.5 1.5 0 01-1.5 1.5h-17A1.5 1.5 0 012 18V8.5z"/>
                  {/* Circular arrows */}
                  <path d="M15.5 12a3.5 3.5 0 01-6.1 2.3" />
                  <path d="M8.5 14a3.5 3.5 0 016.1-2.3" />
                  {/* Arrow heads */}
                  <polyline points="9.2,12.2 9.4,14.5 7.2,14.3" />
                  <polyline points="14.8,15.8 14.6,13.5 16.8,13.7" />
                </svg>
              </button>
            </div>
          </div>

          {/* Bottom: Go Live Button - Aligned with above */}
          <button
            onClick={handleMatch}
            disabled={!cameraReady || isSearching}
            className="flex flex-col items-center gap-1.5 disabled:opacity-50 transition-all active:scale-95"
          >
            <div className="text-center mb-1" style={{ textShadow: '0 1px 3px rgba(0,0,0,0.8)' }}>
              <span className="text-white text-sm font-bold block animate-pulse">Appuie ici pour rejoindre le live</span>
            </div>
            <div className="w-14 h-14 rounded-full bg-white flex items-center justify-center shadow-xl animate-pulse-glow">
              {isSearching ? (
                <span className="w-5 h-5 border-2 border-black/30 border-t-black rounded-full animate-spin"></span>
              ) : (
                <svg className="w-6 h-6 text-black" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
                  <path d="M5 12h14M13 6l6 6-6 6" strokeLinecap="round" strokeLinejoin="round"/>
                </svg>
              )}
            </div>
          </button>
        </div>

        {/* Notifications Panel */}
        {showNotif && (
          <div className="absolute top-14 right-3 w-72 bg-black/95 rounded-2xl border border-white/10 z-30 max-h-[60vh] flex flex-col">
            <div className="p-4 border-b border-white/10 flex justify-between flex-shrink-0">
              <span className="text-white font-bold">Notifications</span>
              <button onClick={() => setShowNotif(false)} className="text-white/50 hover:text-white">✕</button>
            </div>
            <div className="p-4 overflow-y-auto flex-1">
              {notifications.length === 0 ? (
                <p className="text-white/50 text-sm text-center">Aucune notification</p>
              ) : (
                <div className="space-y-3">
                  {notifications.map((notif) => (
                    <button
                      key={notif.id}
                      onClick={() => {
                        setShowNotif(false);
                        if (['new_message', 'follow_request', 'follow_accepted', 'like'].includes(notif.type)) {
                          navigate('/space');
                        }
                      }}
                      className={`w-full text-left p-3 rounded-lg transition-all hover:bg-white/10 ${notif.read ? 'bg-white/5' : 'bg-white/20 border border-white/30'}`}
                    >
                      <p className="text-white/90 text-sm">
                        {notif.type === 'follow_request' && (
                          <>👋 <span className="font-bold">@{notif.from_user?.display_name || notif.from_user_name || 'Bro'}</span> veut te follow</>
                        )}
                        {notif.type === 'follow_accepted' && (
                          <>✅ <span className="font-bold">@{notif.from_user?.display_name || notif.from_user_name || 'Bro'}</span> a accepté ta demande</>
                        )}
                        {notif.type === 'like' && (
                          <>❤️ <span className="font-bold">@{notif.from_user?.display_name || notif.from_user_name || 'Bro'}</span> t'a liké</>
                        )}
                        {notif.type === 'new_message' && (
                          <>💬 <span className="font-bold">@{notif.from_user?.display_name || notif.from_user_name || 'Bro'}</span> t'a envoyé un message</>
                        )}
                        {notif.type === 'livecam_request' && (
                          <>📹 <span className="font-bold">@{notif.from_user?.display_name || notif.from_user_name || 'Bro'}</span> te demande un Live Cam</>
                        )}
                        {notif.type === 'livecam_response' && (
                          notif.accepted
                            ? <>✅ <span className="font-bold">@{notif.from_user?.display_name || notif.from_user_name || 'Bro'}</span> a accepté ton Live Cam</>
                            : <>❌ <span className="font-bold">@{notif.from_user?.display_name || notif.from_user_name || 'Bro'}</span> a refusé ton Live Cam</>
                        )}
                        {notif.type === 'welcome' && '🎉 Bienvenue sur Brozr!'}
                        {!['follow_request', 'follow_accepted', 'like', 'new_message', 'livecam_request', 'livecam_response', 'welcome'].includes(notif.type) && notif.message_preview}
                      </p>
                      <p className="text-white/40 text-xs mt-1">
                        {new Date(notif.created_at).toLocaleDateString('fr-FR', { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' })}
                      </p>
                    </button>
                  ))}
                </div>
              )}
            </div>
          </div>
        )}

        {/* Country Panel */}
        {showCountry && (
          <div className="absolute inset-x-0 bottom-0 bg-black/95 rounded-t-3xl border-t border-white/10 z-30">
            <div className="p-4 border-b border-white/10 flex justify-between items-center">
              <span className="text-white font-bold text-lg">Choisir un pays</span>
              <button onClick={() => setShowCountry(false)} className="w-8 h-8 flex items-center justify-center text-white/50 hover:text-white">✕</button>
            </div>
            <div className="p-4 grid grid-cols-2 gap-2 max-h-[50vh] overflow-y-auto">{renderCountryList()}</div>
          </div>
        )}

        {/* Filters Panel - Now titled "Looking for" - All white theme */}
        {showFilters && (
          <div className="absolute inset-x-0 bottom-0 bg-black/95 rounded-t-3xl border-t border-white/10 z-30 flex flex-col max-h-[60vh]">
            <div className="p-4 border-b border-white/10 flex justify-between items-center flex-shrink-0">
              <span className="text-white font-bold text-lg">Looking for</span>
              <div className="flex items-center gap-3">
                <button onClick={() => { setTempAgeMin(18); setTempAgeMax(60); setTempDistance(400); setTempKinks([]); }} className="text-white/70 text-sm font-medium hover:text-white">Réinitialiser</button>
                <button onClick={() => setShowFilters(false)} className="w-8 h-8 flex items-center justify-center text-white/50 hover:text-white">✕</button>
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

              {/* Distance - Custom slider to match age slider thickness */}
              <div className="mb-5">
                <div className="flex justify-between mb-3">
                  <span className="text-white font-medium">Distance</span>
                  <span className="text-white font-bold">{tempDistance === 400 ? '400+' : tempDistance} km</span>
                </div>
                <div className="relative h-6 flex items-center">
                  {/* Track background */}
                  <div className="absolute w-full h-2 bg-white/10 rounded-full"></div>
                  {/* Active track */}
                  <div className="absolute h-2 bg-white rounded-full" style={{ left: '0%', right: (100 - ((tempDistance - 1) / 399) * 100) + '%' }}></div>
                  {/* Slider */}
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
                  <span className="text-white font-bold">{tempKinks.length}/10</span>
                </div>
                {renderKinkCategories()}
              </div>
            </div>

            {/* Save CTA */}
            <div className="p-4 border-t border-white/10 flex-shrink-0 flex justify-center">
              <button onClick={saveFilters} className="py-3 px-10 bg-white text-black rounded-xl font-bold text-base transition-all active:scale-[0.98]">
                Enregistrer
              </button>
            </div>
          </div>
        )}
      </div>

      {/* Filter Bar - Azar style: smaller, oval buttons covering full width */}
      <div className="bg-black px-3 py-2">
        <div className="flex gap-2">
          {/* Country Filter - Oval, same style as Looking for */}
          <button 
            onClick={() => { if (showCountry) { setShowCountry(false); } else { setShowCountry(true); setShowFilters(false); } }} 
            className="flex-1 flex items-center justify-center gap-1.5 py-2 px-3 bg-white/10 backdrop-blur-sm rounded-full border border-white/20 transition-all active:scale-[0.98]"
          >
            {/* Globe Icon */}
            <svg className="w-4 h-4 text-white flex-shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
              <circle cx="12" cy="12" r="10"/>
              <ellipse cx="12" cy="12" rx="4" ry="10"/>
              <path d="M2 12h20"/>
            </svg>
            <div className="flex flex-col items-center min-w-0">
              {countryLoading ? (
                <span className="text-white/60 font-medium text-xs">...</span>
              ) : (
                <>
                  <span className="text-white font-medium text-xs truncate">
                    {currentCountry.flag} {currentCountry.name}
                  </span>
                  <span className="text-white/50 text-[9px]">Pays préféré</span>
                </>
              )}
            </div>
            <svg className="w-3 h-3 text-white/60 flex-shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M6 9l6 6 6-6" strokeLinecap="round" strokeLinejoin="round"/>
            </svg>
          </button>

          {/* Looking For Filter - Oval, centered text */}
          <button 
            onClick={() => { if (showFilters) { setShowFilters(false); } else { openFilters(); } }} 
            className="flex-1 flex items-center justify-center gap-1.5 py-2 px-3 bg-white/10 backdrop-blur-sm rounded-full border border-white/20 transition-all active:scale-[0.98]"
          >
            {/* Sliders Icon */}
            <svg className="w-4 h-4 text-white flex-shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
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
            <div className="flex flex-col items-center min-w-0">
              <span className="text-white font-medium text-xs">Looking for</span>
              <span className="text-white/50 text-[9px]">Age · Distance · Kinks</span>
            </div>
            {getActiveFiltersCount() > 0 && (
              <span className="min-w-[18px] h-[18px] bg-white rounded-full text-black text-[10px] flex items-center justify-center font-bold flex-shrink-0">{getActiveFiltersCount()}</span>
            )}
            <svg className="w-3 h-3 text-white/60 flex-shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M6 9l6 6 6-6" strokeLinecap="round" strokeLinejoin="round"/>
            </svg>
          </button>
        </div>
      </div>

      {/* Bottom Nav - No border line */}
      <nav className="bg-black px-4 py-3 pb-6">
        <div className="flex justify-around max-w-md mx-auto">
          <button className="flex flex-col items-center gap-1 text-white">
            <svg className="w-6 h-6" fill="currentColor" viewBox="0 0 24 24"><path d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" /></svg>
            <span className="text-xs font-medium">Cam Live</span>
          </button>
          <button onClick={() => navigate('/space')} className="relative flex flex-col items-center gap-1 text-white/50">
            {/* Double bubble icon for Space */}
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
          <button onClick={() => navigate('/play-show')} className="flex flex-col items-center gap-1 text-white/50">
            {/* Play button in square icon for Play Show */}
            <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <rect x="3" y="3" width="18" height="18" rx="3" strokeWidth={1.5} />
              <path d="M10 8l6 4-6 4V8z" strokeWidth={1.5} strokeLinejoin="round" />
            </svg>
            <span className="text-xs font-medium">Play Show</span>
          </button>
        </div>
      </nav>
      </div>
      {/* Account Settings Modal */}
    </div>
  );
};

export default LivePrematch;
