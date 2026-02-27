import React, { useState, useRef, useEffect } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import { useAuth } from '@/contexts/AuthContext';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import axios from 'axios';
import { toast } from 'sonner';

const BACKEND_URL = process.env.REACT_APP_BACKEND_URL;

export const Signup = () => {
  const navigate = useNavigate();
  const { login } = useAuth();
  
  // Form state
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [loading, setLoading] = useState(false);
  
  // Verification state
  const [step, setStep] = useState('form');
  const [verificationCode, setVerificationCode] = useState(['', '', '', '', '', '']);
  const [resendCooldown, setResendCooldown] = useState(0);
  const inputRefs = useRef([]);

  useEffect(() => {
    if (resendCooldown > 0) {
      const timer = setTimeout(() => setResendCooldown(resendCooldown - 1), 1000);
      return () => clearTimeout(timer);
    }
  }, [resendCooldown]);

  const handleSignup = async (e) => {
    e.preventDefault();

    if (password !== confirmPassword) {
      toast.error('Les mots de passe ne correspondent pas');
      return;
    }

    if (password.length < 8) {
      toast.error('Minimum 8 caractères pour le mot de passe');
      return;
    }

    setLoading(true);

    try {
      const response = await axios.post(BACKEND_URL + '/api/auth/signup', {
        email,
        password
      });

      if (response.data.status === 'verification_required') {
        toast.success('Code envoyé !');
        setStep('verification');
        setResendCooldown(60);
      } else {
        const token = response.data.data?.access_token || response.data.token;
        if (token) {
          let userData = null;
          try {
            const meResponse = await axios.get(BACKEND_URL + '/api/auth/me', {
              headers: { Authorization: `Bearer ${token}` }
            });
            userData = meResponse.data.data || meResponse.data;
          } catch (e) {
            // /me might fail if profile not yet created
          }
          login(token, userData);
          navigate('/onboarding');
        }
      }
    } catch (error) {
      toast.error(error.response?.data?.detail || 'Oups, une erreur est survenue');
    } finally {
      setLoading(false);
    }
  };

  const handleCodeChange = (index, value) => {
    if (value && !/^\d$/.test(value)) return;
    
    const newCode = [...verificationCode];
    newCode[index] = value;
    setVerificationCode(newCode);

    if (value && index < 5) {
      inputRefs.current[index + 1]?.focus();
    }
  };

  const handleKeyDown = (index, e) => {
    if (e.key === 'Backspace' && !verificationCode[index] && index > 0) {
      inputRefs.current[index - 1]?.focus();
    }
  };

  const handlePaste = (e) => {
    e.preventDefault();
    const pastedData = e.clipboardData.getData('text').slice(0, 6);
    if (/^\d+$/.test(pastedData)) {
      const newCode = pastedData.split('').concat(Array(6 - pastedData.length).fill(''));
      setVerificationCode(newCode);
      inputRefs.current[Math.min(pastedData.length, 5)]?.focus();
    }
  };

  const handleVerify = async () => {
    const code = verificationCode.join('');
    if (code.length !== 6) {
      toast.error('Entre le code complet');
      return;
    }

    setLoading(true);

    try {
      const response = await axios.post(BACKEND_URL + '/api/auth/verify-email', {
        email,
        code
      });

      const token = response.data.data?.access_token || response.data.token;
      if (token) {
        let userData = null;
        try {
          const meResponse = await axios.get(BACKEND_URL + '/api/auth/me', {
            headers: { Authorization: `Bearer ${token}` }
          });
          userData = meResponse.data.data || meResponse.data;
        } catch (e) {
          // /me might fail if profile not yet created
        }
        login(token, userData);
        navigate('/onboarding');
      }
    } catch (error) {
      toast.error(error.response?.data?.detail || 'Code incorrect');
      setVerificationCode(['', '', '', '', '', '']);
      inputRefs.current[0]?.focus();
    } finally {
      setLoading(false);
    }
  };

  const handleResendCode = async () => {
    if (resendCooldown > 0) return;

    try {
      await axios.post(BACKEND_URL + '/api/auth/resend-code', { email });
      toast.success('Nouveau code envoyé !');
      setResendCooldown(60);
      setVerificationCode(['', '', '', '', '', '']);
      inputRefs.current[0]?.focus();
    } catch (error) {
      toast.error(error.response?.data?.detail || 'Erreur lors de l\'envoi');
    }
  };

  // Verification Step
  if (step === 'verification') {
    return (
      <div className="min-h-screen bg-gradient-to-b from-[#0D0D0D] via-[#080808] to-[#050505] text-white flex items-center justify-center p-6">
        <div className="w-full max-w-md">
          <div className="bg-white/5 backdrop-blur-xl border border-white/10 rounded-2xl p-8">
            <div className="text-center mb-8">
              <span className="text-4xl mb-4 block">📬</span>
              <h2 className="text-2xl font-black mb-2" style={{ fontFamily: 'Unbounded, sans-serif' }} data-testid="verification-title">
                Check tes emails
              </h2>
              <p className="text-white/60 text-sm">
                Un code à 6 chiffres t'attend dans ta boîte
              </p>
              <p className="text-white font-medium mt-2 text-[white]">{email}</p>
            </div>

            <div className="flex justify-center gap-2 mb-6" onPaste={handlePaste}>
              {verificationCode.map((digit, index) => (
                <input
                  key={index}
                  ref={(el) => (inputRefs.current[index] = el)}
                  type="text"
                  inputMode="numeric"
                  maxLength={1}
                  value={digit}
                  onChange={(e) => handleCodeChange(index, e.target.value)}
                  onKeyDown={(e) => handleKeyDown(index, e)}
                  data-testid={`verification-code-${index}`}
                  className="w-12 h-14 text-center text-2xl font-bold bg-white/5 border border-white/20 rounded-lg focus:border-[white] focus:outline-none focus:ring-2 focus:ring-[white]/30 transition-all text-white"
                />
              ))}
            </div>

            <Button
              onClick={handleVerify}
              disabled={loading || verificationCode.join('').length !== 6}
              data-testid="verify-code-btn"
              className="w-full h-14 rounded-xl bg-white text-black hover:bg-white/90 font-bold text-lg transition-all active:scale-[0.98] disabled:opacity-40"
            >
              {loading ? 'Vérification...' : 'C\'est parti'}
            </Button>

            <div className="mt-6 text-center">
              <p className="text-white/40 text-sm mb-2">Pas reçu ?</p>
              <button
                onClick={handleResendCode}
                disabled={resendCooldown > 0}
                data-testid="resend-code-btn"
                className={`text-sm font-medium transition-colors ${
                  resendCooldown > 0 
                    ? 'text-white/30 cursor-not-allowed' 
                    : 'text-[white] hover:text-[white]/80'
                }`}
              >
                {resendCooldown > 0 
                  ? `Renvoyer dans ${resendCooldown}s` 
                  : 'Renvoyer le code'
                }
              </button>
            </div>

            <div className="mt-6 text-center">
              <button
                onClick={() => {
                  setStep('form');
                  setVerificationCode(['', '', '', '', '', '']);
                }}
                className="text-sm text-white/50 hover:text-white/70 transition-colors"
                data-testid="back-to-signup-btn"
              >
                ← Modifier l'email
              </button>
            </div>
          </div>
        </div>
      </div>
    );
  }

  // Signup Form
  return (
    <div className="min-h-screen bg-gradient-to-b from-[#0D0D0D] via-[#080808] to-[#050505] text-white flex items-center justify-center p-6">
      <div className="w-full max-w-md">
        <div className="bg-white/5 backdrop-blur-xl border border-white/10 rounded-2xl p-8">
          <div className="text-center mb-8">
            <span className="text-4xl mb-4 block">🔥</span>
            <h2 className="text-2xl sm:text-3xl font-black mb-2" style={{ fontFamily: 'Unbounded, sans-serif' }} data-testid="signup-title">
              Rejoins nous !
            </h2>
            <p className="text-white/60">Crée ton compte en 30 secondes</p>
          </div>

          {/* Social Signup Buttons */}
          <div className="space-y-3 mb-6">
            <button
              type="button"
              onClick={() => {
                const redirectUrl = window.location.origin + '/auth/callback';
                window.location.href = `https://auth.emergentagent.com/?redirect=${encodeURIComponent(redirectUrl)}`;
              }}
              className="w-full h-12 rounded-xl bg-white text-black hover:bg-white/90 font-medium text-sm transition-all flex items-center justify-center gap-3 active:scale-[0.98]"
            >
              <svg className="w-5 h-5" viewBox="0 0 24 24">
                <path fill="#4285F4" d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"/>
                <path fill="#34A853" d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"/>
                <path fill="#FBBC05" d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"/>
                <path fill="#EA4335" d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"/>
              </svg>
              Continuer avec Google
            </button>
            <button
              type="button"
              onClick={() => toast('Bientôt disponible')}
              className="w-full h-12 rounded-xl bg-white/5 text-white hover:bg-white/10 font-medium text-sm transition-all flex items-center justify-center gap-3 border border-white/10 active:scale-[0.98]"
            >
              <svg className="w-5 h-5" viewBox="0 0 24 24" fill="currentColor">
                <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z"/>
              </svg>
              Continuer avec X
            </button>
            <button
              type="button"
              onClick={() => toast('Bientôt disponible')}
              className="w-full h-12 rounded-xl bg-white/5 text-white hover:bg-white/10 font-medium text-sm transition-all flex items-center justify-center gap-3 border border-white/10 active:scale-[0.98]"
            >
              <svg className="w-5 h-5" viewBox="0 0 24 24" fill="currentColor">
                <path d="M11.944 0A12 12 0 0 0 0 12a12 12 0 0 0 12 12 12 12 0 0 0 12-12A12 12 0 0 0 12 0a12 12 0 0 0-.056 0zm4.962 7.224c.1-.002.321.023.465.14a.506.506 0 0 1 .171.325c.016.093.036.306.02.472-.18 1.898-.962 6.502-1.36 8.627-.168.9-.499 1.201-.82 1.23-.696.065-1.225-.46-1.9-.902-1.056-.693-1.653-1.124-2.678-1.8-1.185-.78-.417-1.21.258-1.91.177-.184 3.247-2.977 3.307-3.23.007-.032.014-.15-.056-.212s-.174-.041-.249-.024c-.106.024-1.793 1.14-5.061 3.345-.48.33-.913.49-1.302.48-.428-.008-1.252-.241-1.865-.44-.752-.245-1.349-.374-1.297-.789.027-.216.325-.437.893-.663 3.498-1.524 5.83-2.529 6.998-3.014 3.332-1.386 4.025-1.627 4.476-1.635z"/>
              </svg>
              Continuer avec Telegram
            </button>
            <button
              type="button"
              onClick={() => toast('Bientôt disponible')}
              className="w-full h-12 rounded-xl bg-white/5 text-white hover:bg-white/10 font-medium text-sm transition-all flex items-center justify-center gap-3 border border-white/10 active:scale-[0.98]"
            >
              <svg className="w-5 h-5" viewBox="0 0 24 24" fill="currentColor">
                <path d="M12.206.793c.99 0 4.347.276 5.93 3.821.529 1.193.403 3.219.299 4.847l-.003.06c-.012.18-.022.345-.03.51.075.045.203.09.401.09.3-.016.659-.12 1.033-.301.165-.088.344-.104.464-.104.182 0 .299.063.336.079a.738.738 0 0 1 .401.61c.031.391-.249.746-.476.99-.39.422-.94.689-1.485.957-.025.012-.045.024-.075.037-.18.088-.39.191-.573.344a2.03 2.03 0 0 0-.42.487c-.317.51-.486 1.181-.486 1.96 0 3.568-2.619 8.22-8.358 8.22-3.217 0-6.163-1.58-7.497-4.085-.381-.72-.573-1.504-.573-2.313 0-.648.123-1.32.366-1.996.244-.676.613-1.356 1.098-2.016.987-1.338 2.437-2.526 4.233-3.259.027-.011.053-.02.079-.028.25-.092.53-.135.818-.135.553 0 1.069.177 1.401.541.203.222.32.497.32.786 0 .345-.154.659-.457.905-.279.226-.69.385-1.175.462-.12.019-.242.035-.363.05-.486.063-.986.128-1.384.396-.224.15-.357.33-.357.47 0 .124.073.325.391.607.328.29.862.587 1.63.854 1.042.362 1.66.621 2.049.95.195.166.345.37.45.604.103.234.15.499.15.781 0 .666-.274 1.27-.77 1.716a2.786 2.786 0 0 1-1.006.57 6.37 6.37 0 0 0 1.444.164c4.372 0 6.623-3.463 6.623-6.579 0-.506.1-.987.315-1.414.076-.152.167-.297.27-.43-.068.003-.127.007-.197.007-.645 0-1.274-.24-1.73-.626a2.064 2.064 0 0 1-.705-1.56c0-.06.005-.12.013-.177l.007-.019c.043-.41.132-.778.229-1.119.038-.135.078-.27.119-.397-.093-.003-.198-.003-.3-.003-.618 0-1.091.093-1.384.18-.045.015-.075.024-.105.033-.04.012-.075.017-.105.017-.159 0-.25-.103-.268-.15a.362.362 0 0 1-.015-.075c0-.121.063-.255.195-.369.252-.217.749-.453 1.53-.586.233-.04.479-.06.731-.06z"/>
              </svg>
              Continuer avec Snap
            </button>
          </div>

          {/* Separator */}
          <div className="flex items-center gap-3 mb-6">
            <div className="flex-1 h-px bg-white/10"></div>
            <span className="text-white/40 text-xs uppercase tracking-wider">ou par email</span>
            <div className="flex-1 h-px bg-white/10"></div>
          </div>

          <form onSubmit={handleSignup} className="space-y-5">
            <div>
              <Label htmlFor="email" className="text-white/60 text-sm">
                Ton email
              </Label>
              <Input
                id="email"
                type="email"
                data-testid="signup-email-input"
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
                data-testid="signup-password-input"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className="mt-1 bg-white/5 border-white/10 focus:border-[white] h-12 rounded-xl text-white placeholder:text-white/30"
                placeholder="Min. 8 caractères"
                required
              />
            </div>

            <div>
              <Label htmlFor="confirmPassword" className="text-white/60 text-sm">
                Confirme ton mot de passe
              </Label>
              <Input
                id="confirmPassword"
                type="password"
                data-testid="signup-confirm-password-input"
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
                className="mt-1 bg-white/5 border-white/10 focus:border-[white] h-12 rounded-xl text-white placeholder:text-white/30"
                placeholder="Confirme ici"
                required
              />
            </div>

            <Button
              type="submit"
              data-testid="signup-submit-btn"
              disabled={loading}
              className="w-full h-14 rounded-xl bg-white text-black hover:bg-white/90 font-bold text-lg transition-all active:scale-[0.98] disabled:opacity-40"
            >
              {loading ? 'Création...' : 'Créer mon compte'}
            </Button>
          </form>

          <p className="text-white/30 text-xs text-center mt-4">
            Gratuit · Tes données restent privées
          </p>

          <div className="mt-6 text-center text-sm text-white/60">
            Déjà un compte ?{' '}
            <Link to="/login" className="text-[white] hover:text-[white]/80 font-medium" data-testid="signup-login-link">
              Connecte-toi
            </Link>
          </div>
        </div>
      </div>
    </div>
  );
};

export default Signup;
