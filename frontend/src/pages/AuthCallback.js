import React, { useState, useEffect, useRef } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { useAuth } from '@/contexts/AuthContext';
import axios from 'axios';

const BACKEND_URL = process.env.REACT_APP_BACKEND_URL;

export const AuthCallback = () => {
  const location = useLocation();
  const navigate = useNavigate();
  const { login } = useAuth();
  const [error, setError] = useState(null);
  const hasProcessed = useRef(false);

  useEffect(() => {
    // REMINDER: DO NOT HARDCODE THE URL, OR ADD ANY FALLBACKS OR REDIRECT URLS, THIS BREAKS THE AUTH
    const processAuth = async () => {
      // Prevent double processing in StrictMode
      if (hasProcessed.current) return;
      hasProcessed.current = true;
      
      const hash = location.hash.substring(1);
      const params = new URLSearchParams(hash);
      const sessionId = params.get('session_id');

      if (!sessionId) {
        setError('No session ID found');
        return;
      }

      try {
        const response = await axios.post(
          `${BACKEND_URL}/api/auth/oauth/session`,
          { session_id: sessionId },
          { withCredentials: true }
        );

        const user = response.data.user;
        const token = response.data.token;
        
        // CRITICAL: Use the login function from AuthContext
        // This clears all previous user data before setting new user
        login(token, user);
        
        if (!user.onboarding_complete) {
          navigate('/onboarding', { state: { user }, replace: true });
        } else {
          navigate('/live-prematch', { state: { user }, replace: true });
        }
      } catch (err) {
        console.error('Auth callback error:', err);
        setError('Authentication failed');
        setTimeout(() => navigate('/login'), 2000);
      }
    };

    processAuth();
  }, [location, navigate, login]);

  if (error) {
    return (
      <div className="min-h-screen bg-[#050505] flex items-center justify-center text-white">
        <div className="text-center">
          <p className="text-red-500 mb-4">{error}</p>
          <p className="text-white/60">Redirecting...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-[#050505] flex items-center justify-center">
      <div className="text-center text-white">
        <div className="animate-spin rounded-full h-16 w-16 border-t-2 border-b-2 border-white mx-auto mb-4"></div>
        <p>Connexion à Brozr...</p>
      </div>
    </div>
  );
};

export default AuthCallback;
