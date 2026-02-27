import React, { useState, useEffect } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import { useAuth } from '@/contexts/AuthContext';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import axios from 'axios';
import { toast } from 'sonner';

const BACKEND_URL = process.env.REACT_APP_BACKEND_URL;

export const Login = () => {
  const navigate = useNavigate();
  const { login } = useAuth();
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [rememberMe, setRememberMe] = useState(false);
  const [loading, setLoading] = useState(false);
  
  // Forgot password state
  const [showForgotPassword, setShowForgotPassword] = useState(false);
  const [forgotEmail, setForgotEmail] = useState('');
  const [resetCode, setResetCode] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [forgotStep, setForgotStep] = useState(1); // 1: email, 2: code + new password
  const [forgotLoading, setForgotLoading] = useState(false);

  // Load saved email on mount + purge any previously saved password
  useEffect(() => {
    localStorage.removeItem('brozr_saved_password');

    const savedEmail = localStorage.getItem('brozr_saved_email');
    const savedRemember = localStorage.getItem('brozr_remember_me');

    if (savedRemember === 'true' && savedEmail) {
      setEmail(savedEmail);
      setRememberMe(true);
    }
  }, []);

  const handleLogin = async (e) => {
    e.preventDefault();
    setLoading(true);

    try {
      const response = await axios.post(BACKEND_URL + '/api/auth/login', {
        email,
        password
      });

      const token = response.data.data?.access_token || response.data.token;
      if (token) {
        // Save email if "remember me" is checked (never save password)
        if (rememberMe) {
          localStorage.setItem('brozr_saved_email', email);
          localStorage.setItem('brozr_remember_me', 'true');
        } else {
          localStorage.removeItem('brozr_saved_email');
          localStorage.removeItem('brozr_remember_me');
        }

        // Fetch user profile to check onboarding status
        let userData = null;
        let profileData = null;
        try {
          const authHeaders = { Authorization: `Bearer ${token}` };
          const [meResponse, profileResponse] = await Promise.allSettled([
            axios.get(BACKEND_URL + '/api/auth/me', { headers: authHeaders }),
            axios.get(BACKEND_URL + '/api/users/me', { headers: authHeaders }),
          ]);
          if (meResponse.status === 'fulfilled') {
            userData = meResponse.value.data.data || meResponse.value.data;
          }
          if (profileResponse.status === 'fulfilled') {
            profileData = profileResponse.value.data.data || profileResponse.value.data;
            userData = { ...userData, ...profileData };
          }
        } catch (e) {
          // continue anyway
        }

        login(token, userData);

        if (!profileData?.onboarding_complete) {
          navigate('/onboarding');
        } else {
          navigate('/live-prematch');
        }
      }
    } catch (error) {
      toast.error(error.response?.data?.detail || 'Email ou mot de passe incorrect');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-b from-[#0D0D0D] via-[#080808] to-[#050505] text-white flex items-center justify-center p-6">
      <div className="w-full max-w-md">
        <div className="bg-white/5 backdrop-blur-xl border border-white/10 rounded-2xl p-8">
          <div className="text-center mb-8">
            <span className="text-4xl mb-4 block">👋</span>
            <h2 className="text-2xl sm:text-3xl font-black mb-2" style={{ fontFamily: 'Unbounded, sans-serif' }} data-testid="login-title">
              Reconnecte-toi !
            </h2>
            <p className="text-white/60">Tes Bros. sont déjà en live</p>
          </div>

          <form onSubmit={handleLogin} className="space-y-5">
            <div>
              <Label htmlFor="email" className="text-white/60 text-sm">
                Ton email
              </Label>
              <Input
                id="email"
                type="email"
                data-testid="login-email-input"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                className="mt-1 bg-white/5 border-white/10 focus:border-[white] h-12 rounded-xl text-white placeholder:text-white/30"
                placeholder="ton@email.com"
                required
              />
            </div>

            <div>
              <Label htmlFor="password" className="text-white/60 text-sm">
                Mot de passe
              </Label>
              <Input
                id="password"
                type="password"
                data-testid="login-password-input"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className="mt-1 bg-white/5 border-white/10 focus:border-[white] h-12 rounded-xl text-white placeholder:text-white/30"
                placeholder="Ton mot de passe"
                required
              />
            </div>

            {/* Remember Me Checkbox */}
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <input
                  type="checkbox"
                  id="rememberMe"
                  checked={rememberMe}
                  onChange={(e) => setRememberMe(e.target.checked)}
                  className="w-4 h-4 rounded border-white/20 bg-white/5 text-[white] focus:ring-[white] focus:ring-offset-0 cursor-pointer"
                  data-testid="login-remember-me"
                />
                <Label htmlFor="rememberMe" className="text-white/60 text-sm cursor-pointer">
                  Se souvenir de moi
                </Label>
              </div>
              <button
                type="button"
                onClick={() => {
                  setShowForgotPassword(true);
                  setForgotEmail(email);
                  setForgotStep(1);
                }}
                className="text-[white] hover:text-[white]/80 text-sm"
                data-testid="forgot-password-link"
              >
                Mot de passe oublié ?
              </button>
            </div>

            <Button
              type="submit"
              data-testid="login-submit-btn"
              disabled={loading}
              className="w-full h-14 rounded-xl bg-white text-black hover:bg-white/90 font-bold text-lg transition-all active:scale-[0.98] disabled:opacity-40"
            >
              {loading ? 'Connexion...' : 'Se connecter'}
            </Button>
          </form>

          <div className="mt-6 text-center text-sm text-white/60">
            Pas encore de compte ?{' '}
            <Link to="/signup" className="text-[white] hover:text-[white]/80 font-medium" data-testid="login-signup-link">
              Rejoins-nous
            </Link>
          </div>
        </div>
      </div>
      
      {/* Forgot Password Modal */}
      {showForgotPassword && (
        <div className="fixed inset-0 bg-black/80 backdrop-blur-sm z-50 flex items-center justify-center p-6">
          <div className="bg-[#111] border border-white/10 rounded-2xl p-6 w-full max-w-md">
            <div className="flex justify-between items-center mb-6">
              <h3 className="text-xl font-bold text-white">
                {forgotStep === 1 ? 'Mot de passe oublié' : 'Nouveau mot de passe'}
              </h3>
              <button 
                onClick={() => {
                  setShowForgotPassword(false);
                  setForgotStep(1);
                  setResetCode('');
                  setNewPassword('');
                }}
                className="text-white/50 hover:text-white"
              >
                ✕
              </button>
            </div>
            
            {forgotStep === 1 ? (
              // Step 1: Enter email
              <div className="space-y-4">
                <p className="text-white/60 text-sm">
                  Entre ton email et on t'envoie un code de réinitialisation.
                </p>
                <div>
                  <Label className="text-white/60 text-sm">Email</Label>
                  <Input
                    type="email"
                    value={forgotEmail}
                    onChange={(e) => setForgotEmail(e.target.value)}
                    className="mt-1 bg-white/5 border-white/10 focus:border-[white] h-12 rounded-xl text-white"
                    placeholder="ton@email.com"
                    data-testid="forgot-email-input"
                  />
                </div>
                <Button
                  onClick={async () => {
                    if (!forgotEmail) {
                      toast.error('Entre ton email');
                      return;
                    }
                    setForgotLoading(true);
                    try {
                      await axios.post(`${BACKEND_URL}/api/auth/forgot-password`, { email: forgotEmail });
                      toast.success('Code envoyé par email !');
                      setForgotStep(2);
                    } catch (err) {
                      toast.error('Erreur lors de l\'envoi');
                    } finally {
                      setForgotLoading(false);
                    }
                  }}
                  disabled={forgotLoading}
                  className="w-full h-12 rounded-xl bg-white text-black hover:bg-white/90 font-bold"
                  data-testid="forgot-send-code-btn"
                >
                  {forgotLoading ? 'Envoi...' : 'Envoyer le code'}
                </Button>
              </div>
            ) : (
              // Step 2: Enter code + new password
              <div className="space-y-4">
                <p className="text-white/60 text-sm">
                  Entre le code reçu par email et ton nouveau mot de passe.
                </p>
                <div>
                  <Label className="text-white/60 text-sm">Code de vérification</Label>
                  <Input
                    type="text"
                    value={resetCode}
                    onChange={(e) => setResetCode(e.target.value)}
                    className="mt-1 bg-white/5 border-white/10 focus:border-[white] h-12 rounded-xl text-white text-center text-2xl tracking-widest"
                    placeholder="000000"
                    maxLength={6}
                    data-testid="forgot-code-input"
                  />
                </div>
                <div>
                  <Label className="text-white/60 text-sm">Nouveau mot de passe</Label>
                  <Input
                    type="password"
                    value={newPassword}
                    onChange={(e) => setNewPassword(e.target.value)}
                    className="mt-1 bg-white/5 border-white/10 focus:border-[white] h-12 rounded-xl text-white"
                    placeholder="Ton nouveau mot de passe"
                    data-testid="forgot-new-password-input"
                  />
                </div>
                <Button
                  onClick={async () => {
                    if (!resetCode || !newPassword) {
                      toast.error('Remplis tous les champs');
                      return;
                    }
                    if (newPassword.length < 6) {
                      toast.error('Mot de passe trop court (min 6 caractères)');
                      return;
                    }
                    setForgotLoading(true);
                    try {
                      await axios.post(`${BACKEND_URL}/api/auth/reset-password`, {
                        email: forgotEmail,
                        code: resetCode,
                        new_password: newPassword
                      });
                      toast.success('Mot de passe modifié ! 🎉');
                      setShowForgotPassword(false);
                      setForgotStep(1);
                      setResetCode('');
                      setNewPassword('');
                      setPassword(''); // Clear password field to force re-entry
                    } catch (err) {
                      toast.error(err.response?.data?.detail || 'Code invalide ou expiré');
                    } finally {
                      setForgotLoading(false);
                    }
                  }}
                  disabled={forgotLoading}
                  className="w-full h-12 rounded-xl bg-white text-black hover:bg-white/90 font-bold"
                  data-testid="forgot-reset-btn"
                >
                  {forgotLoading ? 'Modification...' : 'Changer le mot de passe'}
                </Button>
                <button
                  onClick={() => setForgotStep(1)}
                  className="w-full text-center text-white/50 hover:text-white text-sm"
                >
                  ← Retour
                </button>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
};

export default Login;
