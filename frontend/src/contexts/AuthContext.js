import React, { createContext, useContext, useState, useEffect } from 'react';
import { getAuthToken, setAuthToken, clearAllUserData, getCurrentUser } from '@/utils/auth';

const AuthContext = createContext(null);

export const useAuth = () => {
  const context = useContext(AuthContext);
  if (!context) throw new Error('useAuth must be used within AuthProvider');
  return context;
};

export const AuthProvider = ({ children }) => {
  const [user, setUser] = useState(null);
  const [token, setToken] = useState(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const checkAuth = async () => {
      try {
        // First check for stored token
        const storedToken = getAuthToken();
        if (storedToken) {
          setToken(storedToken);
        }
        
        const authHeaders = storedToken ? { 'Authorization': `Bearer ${storedToken}` } : {};
        const response = await fetch(`${process.env.REACT_APP_BACKEND_URL}/api/auth/me`, {
          credentials: 'include',
          headers: authHeaders,
        });
        if (response.ok) {
          const authData = await response.json();
          let userData = authData.data || authData;
          // Also fetch profile for onboarding_complete etc.
          try {
            const profileResp = await fetch(`${process.env.REACT_APP_BACKEND_URL}/api/users/me`, {
              headers: authHeaders,
            });
            if (profileResp.ok) {
              const profileJson = await profileResp.json();
              const profileData = profileJson.data || profileJson;
              userData = { ...userData, ...profileData };
            }
          } catch (e) { /* profile may not exist yet */ }
          setUser(userData);
        } else {
          // Token invalid or expired - clear all user data
          clearAllUserData();
        }
      } catch (error) {
        console.error('Auth check failed:', error);
      } finally {
        setLoading(false);
      }
    };
    checkAuth();
  }, []);

  const login = (newToken, userData) => {
    // CRITICAL: Clear any existing user data before setting new user
    // This prevents data leakage between accounts
    clearAllUserData();
    
    if (newToken) {
      setAuthToken(newToken);
      setToken(newToken);
    }
    if (userData) {
      localStorage.setItem('brozr_user', JSON.stringify(userData));
    }
    setUser(userData);
  };

  const logout = async () => {
    try {
      await fetch(`${process.env.REACT_APP_BACKEND_URL}/api/auth/logout`, {
        method: 'POST',
        credentials: 'include'
      });
    } catch (error) {
      console.error('Logout error:', error);
    }
    // CRITICAL: Clear ALL user data on logout
    clearAllUserData();
    setToken(null);
    setUser(null);
  };

  const updateUser = (userData) => {
    setUser(userData);
    if (userData) {
      localStorage.setItem('brozr_user', JSON.stringify(userData));
    }
  };

  return (
    <AuthContext.Provider value={{ user, token, loading, login, logout, updateUser }}>
      {children}
    </AuthContext.Provider>
  );
};
