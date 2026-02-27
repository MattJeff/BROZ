import React from 'react';
import '@/App.css';
import '@/i18n';
import { BrowserRouter, Routes, Route, Navigate, useLocation } from 'react-router-dom';
import { AuthProvider, useAuth } from '@/contexts/AuthContext';
import { Toaster } from '@/components/ui/sonner';
import GlobalLiveCamListener from '@/components/GlobalLiveCamListener';
import Hero from '@/components/Hero';
import Login from '@/pages/Login';
import Signup from '@/pages/Signup';
import Onboarding from '@/pages/Onboarding';
import AuthCallback from '@/pages/AuthCallback';
import LivePrematch from '@/pages/LivePrematch';
import VideoCall from '@/pages/VideoCall';
import SafetyScreen from '@/pages/SafetyScreen';
import Live from '@/pages/Live';
import Space from '@/pages/Space';
import Profile from '@/pages/Profile';
import PlayShow from '@/pages/PlayShow';
import AdminDashboard from '@/pages/AdminDashboard';

const ProtectedRoute = ({ children }) => {
  const { user, loading } = useAuth();

  if (loading) {
    return (
      <div className="min-h-screen bg-[#050505] flex items-center justify-center">
        <div className="animate-spin rounded-full h-16 w-16 border-t-2 border-b-2 border-blue-500"></div>
      </div>
    );
  }

  return user ? children : <Navigate to="/login" />;
};

const OnboardingRoute = ({ children }) => {
  const { user, loading } = useAuth();

  if (loading) {
    return (
      <div className="min-h-screen bg-[#050505] flex items-center justify-center">
        <div className="animate-spin rounded-full h-16 w-16 border-t-2 border-b-2 border-blue-500"></div>
      </div>
    );
  }

  if (!user) return <Navigate to="/login" />;
  if (user.onboarding_complete) return <Navigate to="/live-prematch" />;
  return children;
};

function AppContent() {
  const location = useLocation();
  
  // Check for auth callback synchronously
  if (location.hash?.includes('session_id=')) {
    return <AuthCallback />;
  }

  return (
    <div className="App">
      <Routes>
        <Route path="/" element={<Hero />} />
        <Route path="/login" element={<Login />} />
        <Route path="/signup" element={<Signup />} />
        <Route path="/auth/callback" element={<AuthCallback />} />
        <Route
          path="/onboarding"
          element={
            <OnboardingRoute>
              <Onboarding />
            </OnboardingRoute>
          }
        />
        <Route
          path="/live-prematch"
          element={
            <ProtectedRoute>
              <LivePrematch />
            </ProtectedRoute>
          }
        />
        <Route
          path="/safety"
          element={
            <ProtectedRoute>
              <SafetyScreen />
            </ProtectedRoute>
          }
        />
        <Route
          path="/video-call"
          element={
            <ProtectedRoute>
              <VideoCall />
            </ProtectedRoute>
          }
        />
        <Route
          path="/live"
          element={
            <ProtectedRoute>
              <Live />
            </ProtectedRoute>
          }
        />
        <Route
          path="/space"
          element={
            <ProtectedRoute>
              <Space />
            </ProtectedRoute>
          }
        />
        <Route
          path="/profile"
          element={
            <ProtectedRoute>
              <Profile />
            </ProtectedRoute>
          }
        />
        <Route
          path="/play-show"
          element={
            <ProtectedRoute>
              <PlayShow />
            </ProtectedRoute>
          }
        />
        <Route
          path="/admin"
          element={
            <ProtectedRoute>
              <AdminDashboard />
            </ProtectedRoute>
          }
        />
      </Routes>
      <GlobalLiveCamListener />
      <Toaster position="top-center" richColors />
    </div>
  );
}

function App() {
  return (
    <AuthProvider>
      <BrowserRouter>
        <AppContent />
      </BrowserRouter>
    </AuthProvider>
  );
}

export default App;
