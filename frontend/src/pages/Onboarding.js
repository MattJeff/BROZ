import React, { useState, useRef, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '@/contexts/AuthContext';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import axios from 'axios';
import { toast } from 'sonner';
import { getAuthToken } from '@/utils/auth';
import { KINKS_FLAT } from '@/utils/kinks';

const BACKEND_URL = process.env.REACT_APP_BACKEND_URL;

const COUNTRY_LIST = [
  { code: 'FR', name: 'France', flag: '\u{1F1EB}\u{1F1F7}' },
  { code: 'BE', name: 'Belgique', flag: '\u{1F1E7}\u{1F1EA}' },
  { code: 'CH', name: 'Suisse', flag: '\u{1F1E8}\u{1F1ED}' },
  { code: 'CA', name: 'Canada', flag: '\u{1F1E8}\u{1F1E6}' },
  { code: 'US', name: '\u00C9tats-Unis', flag: '\u{1F1FA}\u{1F1F8}' },
  { code: 'UK', name: 'Royaume-Uni', flag: '\u{1F1EC}\u{1F1E7}' },
  { code: 'DE', name: 'Allemagne', flag: '\u{1F1E9}\u{1F1EA}' },
  { code: 'ES', name: 'Espagne', flag: '\u{1F1EA}\u{1F1F8}' },
  { code: 'IT', name: 'Italie', flag: '\u{1F1EE}\u{1F1F9}' },
  { code: 'NL', name: 'Pays-Bas', flag: '\u{1F1F3}\u{1F1F1}' },
  { code: 'PT', name: 'Portugal', flag: '\u{1F1F5}\u{1F1F9}' },
  { code: 'AT', name: 'Autriche', flag: '\u{1F1E6}\u{1F1F9}' },
];

// Kink categories for filters - use centralized data
const KINK_DATA = KINKS_FLAT;

function KinkButton({ label, selected, onClick }) {
  const cls = selected 
    ? "px-3 py-1.5 rounded-full text-sm font-medium bg-white text-black shadow-lg transition-all"
    : "px-3 py-1.5 rounded-full text-sm font-medium bg-white/5 text-white/70 hover:bg-white/10 border border-white/10 transition-all";
  return (
    <button type="button" onClick={onClick} className={cls}>
      {label}
    </button>
  );
}

function KinkCategory({ category, selectedKinks, onToggle }) {
  const buttons = [];
  for (let i = 0; i < category.items.length; i++) {
    const item = category.items[i];
    buttons.push(
      <KinkButton
        key={item}
        label={item}
        selected={selectedKinks.indexOf(item) !== -1}
        onClick={() => onToggle(item)}
      />
    );
  }
  return (
    <div className="mb-5">
      <h4 className="text-white/50 text-xs font-medium mb-2 uppercase tracking-wide">
        <span className="mr-1">{category.emoji}</span>
        {category.cat}
      </h4>
      <div className="flex flex-wrap gap-2">{buttons}</div>
    </div>
  );
}

export const Onboarding = () => {
  const navigate = useNavigate();
  const { updateUser } = useAuth();
  const fileInputRef = useRef(null);
  
  const [step, setStep] = useState(1);
  const [birthDay, setBirthDay] = useState('');
  const [birthMonth, setBirthMonth] = useState('');
  const [birthYear, setBirthYear] = useState('');
  const [ageError, setAgeError] = useState(false);
  const [displayName, setDisplayName] = useState('');
  const [bio, setBio] = useState('');
  const [selectedKinks, setSelectedKinks] = useState([]);
  const [profilePhoto, setProfilePhoto] = useState(null);
  const [photoPreview, setPhotoPreview] = useState(null);
  const [selectedCountry, setSelectedCountry] = useState('');
  const [loading, setLoading] = useState(false);
  const [uploadingPhoto, setUploadingPhoto] = useState(false);
  const [pseudoAvailable, setPseudoAvailable] = useState(null);
  const [checkingPseudo, setCheckingPseudo] = useState(false);

  // Debounced pseudo availability check
  const checkPseudoAvailability = useCallback(async (pseudo) => {
    if (!pseudo || pseudo.length < 2) {
      setPseudoAvailable(null);
      return;
    }
    
    setCheckingPseudo(true);
    try {
      const token = getAuthToken();
      const response = await axios.get(
        `${BACKEND_URL}/api/users/check-pseudo?name=${encodeURIComponent(pseudo)}`,
        { headers: { Authorization: `Bearer ${token}` } }
      );
      setPseudoAvailable(response.data.data?.available ?? response.data.available);
    } catch (error) {
      setPseudoAvailable(null);
    } finally {
      setCheckingPseudo(false);
    }
  }, []);

  // Debounce the pseudo check
  useEffect(() => {
    const timer = setTimeout(() => {
      checkPseudoAvailability(displayName);
    }, 500);
    return () => clearTimeout(timer);
  }, [displayName, checkPseudoAvailability]);

  // Auto-detect country from IP
  useEffect(() => {
    const detectCountry = async () => {
      try {
        const res = await fetch('https://ipapi.co/json/');
        const data = await res.json();
        const code = data.country_code === 'GB' ? 'UK' : data.country_code;
        if (COUNTRY_LIST.find(c => c.code === code)) {
          setSelectedCountry(code);
        }
      } catch (e) {
        // silent fallback
      }
    };
    detectCountry();
  }, []);

  const calculateAge = () => {
    const today = new Date();
    const birth = new Date(parseInt(birthYear), parseInt(birthMonth) - 1, parseInt(birthDay));
    let age = today.getFullYear() - birth.getFullYear();
    const m = today.getMonth() - birth.getMonth();
    if (m < 0 || (m === 0 && today.getDate() < birth.getDate())) {
      age = age - 1;
    }
    return age;
  };

  const handleAgeVerification = () => {
    const age = calculateAge();
    if (age < 18) {
      setAgeError(true);
      toast.error('Tu dois avoir 18 ans ou plus pour rejoindre Brozr');
      return;
    }
    setAgeError(false);
    setStep(2);
  };

  const handlePhotoSelect = (e) => {
    const file = e.target.files[0];
    if (file) {
      if (file.size > 5 * 1024 * 1024) {
        toast.error('La photo doit faire moins de 5 Mo');
        return;
      }
      setProfilePhoto(file);
      const reader = new FileReader();
      reader.onloadend = () => {
        setPhotoPreview(reader.result);
      };
      reader.readAsDataURL(file);
    }
  };

  const handleKinkToggle = (kink) => {
    const idx = selectedKinks.indexOf(kink);
    if (idx !== -1) {
      const newKinks = selectedKinks.filter(k => k !== kink);
      setSelectedKinks(newKinks);
    } else if (selectedKinks.length < 10) {
      setSelectedKinks([...selectedKinks, kink]);
    } else {
      toast.error('Maximum 10 kinks');
    }
  };

  const uploadPhoto = async () => {
    if (!profilePhoto) return null;
    
    setUploadingPhoto(true);
    try {
      const token = getAuthToken();
      const formData = new FormData();
      formData.append('file', profilePhoto);
      
      const response = await axios.post(
        BACKEND_URL + '/api/users/photo',
        formData,
        {
          headers: {
            'Authorization': 'Bearer ' + token,
            'Content-Type': 'multipart/form-data'
          }
        }
      );
      return response.data.photo_url;
    } catch (error) {
      console.error('Photo upload error:', error);
      toast.error('Erreur lors de l\'upload de la photo');
      return null;
    } finally {
      setUploadingPhoto(false);
    }
  };

  const handleProfileSetup = async () => {
    if (!displayName.trim()) {
      toast.error('Choisis un pseudo pour continuer');
      return;
    }
    if (pseudoAvailable === false) {
      toast.error('Ce pseudo est déjà pris');
      return;
    }
    
    setLoading(true);
    
    try {
      const token = getAuthToken();
      
      // Upload photo first if selected
      let photoUrl = null;
      if (profilePhoto) {
        photoUrl = await uploadPhoto();
      }
      
      const birthDate = birthYear + '-' + birthMonth.padStart(2, '0') + '-' + birthDay.padStart(2, '0');
      
      const profileData = {
        display_name: displayName,
        bio: bio || '',
        kinks: selectedKinks,
        birth_date: birthDate,
        country: selectedCountry,
      };

      const response = await axios.post(
        BACKEND_URL + '/api/users/onboarding',
        profileData,
        { headers: { Authorization: 'Bearer ' + token } }
      );

      updateUser(response.data.data || response.data);
      navigate('/live');
    } catch (error) {
      toast.error('Oups, une erreur est survenue');
    } finally {
      setLoading(false);
    }
  };

  const renderKinkCategories = () => {
    const categories = [];
    for (let i = 0; i < KINK_DATA.length; i++) {
      categories.push(
        <KinkCategory
          key={KINK_DATA[i].cat}
          category={KINK_DATA[i]}
          selectedKinks={selectedKinks}
          onToggle={handleKinkToggle}
        />
      );
    }
    return categories;
  };

  return (
    <div className="min-h-screen bg-gradient-to-b from-[#0D0D0D] via-[#080808] to-[#050505] text-white p-4 sm:p-6 flex items-center justify-center">
      <div className="w-full max-w-lg">
        
        {/* Progress indicator - 2 steps */}
        <div className="flex justify-center gap-3 mb-8">
          <div className={step >= 1 ? "h-1.5 w-20 rounded-full bg-white" : "h-1.5 w-20 rounded-full bg-white/20"}></div>
          <div className={step >= 2 ? "h-1.5 w-20 rounded-full bg-white" : "h-1.5 w-20 rounded-full bg-white/20"}></div>
        </div>

        {/* Step 1: Age Verification */}
        {step === 1 && (
          <div className="bg-white/5 backdrop-blur-xl border border-white/10 rounded-2xl p-6 sm:p-8" data-testid="age-verification-step">
            <div className="text-center mb-8">
              <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-white/10 flex items-center justify-center">
                <svg className="w-8 h-8 text-white" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                  <path d="M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z"/>
                  <path d="M12 6v6l4 2"/>
                  <path d="M8 12h8" strokeWidth="2"/>
                </svg>
              </div>
              <h2 className="text-2xl sm:text-3xl font-black mb-2" style={{ fontFamily: 'Unbounded, sans-serif' }}>
                Confirme ton âge
              </h2>
              <p className="text-white/60">
                Brozr est réservé aux adultes.
              </p>
              <p className="text-white/40 text-sm">Tes données restent privées.</p>
            </div>

            <div className="space-y-6">
              <div className="grid grid-cols-3 gap-3">
                <div>
                  <Label htmlFor="day" className="text-white/60 text-sm">Jour</Label>
                  <Input
                    id="day"
                    type="text"
                    inputMode="numeric"
                    pattern="[0-9]*"
                    data-testid="birthdate-day-input"
                    placeholder="JJ"
                    maxLength={2}
                    value={birthDay}
                    onChange={(e) => setBirthDay(e.target.value.replace(/\D/g, ''))}
                    className="mt-1 bg-white/5 border-white/10 focus:border-white h-14 rounded-xl text-white text-center text-lg font-medium"
                  />
                </div>
                <div>
                  <Label htmlFor="month" className="text-white/60 text-sm">Mois</Label>
                  <Input
                    id="month"
                    type="text"
                    inputMode="numeric"
                    pattern="[0-9]*"
                    data-testid="birthdate-month-input"
                    placeholder="MM"
                    maxLength={2}
                    value={birthMonth}
                    onChange={(e) => setBirthMonth(e.target.value.replace(/\D/g, ''))}
                    className="mt-1 bg-white/5 border-white/10 focus:border-white h-14 rounded-xl text-white text-center text-lg font-medium"
                  />
                </div>
                <div>
                  <Label htmlFor="year" className="text-white/60 text-sm">Année</Label>
                  <Input
                    id="year"
                    type="text"
                    inputMode="numeric"
                    pattern="[0-9]*"
                    data-testid="birthdate-year-input"
                    placeholder="AAAA"
                    maxLength={4}
                    value={birthYear}
                    onChange={(e) => setBirthYear(e.target.value.replace(/\D/g, ''))}
                    className="mt-1 bg-white/5 border-white/10 focus:border-white h-14 rounded-xl text-white text-center text-lg font-medium"
                  />
                </div>
              </div>

              {ageError && (
                <div className="p-4 bg-red-500/10 border border-red-500/20 rounded-xl text-center" data-testid="age-error-message">
                  <p className="text-red-400 text-sm">Tu dois avoir 18 ans ou plus pour nous rejoindre</p>
                </div>
              )}

              <Button
                data-testid="age-verification-continue-btn"
                onClick={handleAgeVerification}
                disabled={!birthDay || !birthMonth || !birthYear}
                className="w-full h-14 rounded-xl bg-white text-black hover:bg-white/90 font-bold text-lg transition-all active:scale-[0.98] disabled:opacity-40"
              >
                C'est parti
              </Button>
            </div>
          </div>
        )}

        {/* Step 2: Profile + Kinks (merged) */}
        {step === 2 && (
          <div className="bg-white/5 backdrop-blur-xl border border-white/10 rounded-2xl p-4 sm:p-6 max-h-[85vh] overflow-hidden flex flex-col mt-2" data-testid="profile-setup-step">
            <div className="text-center mb-3 flex-shrink-0">
              <h2 className="text-2xl sm:text-3xl font-black mb-1" style={{ fontFamily: 'Unbounded, sans-serif' }}>
                Ton profil
              </h2>
              <p className="text-white/50 text-sm">Montre qui tu es vraiment</p>
            </div>

            <div className="overflow-y-auto flex-1 pr-2 space-y-4" style={{ scrollbarWidth: 'thin', scrollbarColor: 'rgba(255,255,255,0.2) transparent' }}>
              
              {/* Photo Upload */}
              <div className="flex flex-col items-center">
                <input
                  type="file"
                  ref={fileInputRef}
                  onChange={handlePhotoSelect}
                  accept="image/*"
                  className="hidden"
                  data-testid="photo-input"
                />
                <button
                  type="button"
                  onClick={() => fileInputRef.current.click()}
                  className="relative group"
                  data-testid="photo-upload-btn"
                >
                  <div className={`w-20 h-20 rounded-full border-2 flex items-center justify-center overflow-hidden transition-all ${
                    photoPreview ? 'border-white' : 'border-white/30 hover:border-white/50'
                  }`}>
                    {photoPreview ? (
                      <img src={photoPreview} alt="Preview" className="w-full h-full object-cover" />
                    ) : (
                      <div className="text-center">
                        <svg className="w-8 h-8 mx-auto text-white/40" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M12 4v16m8-8H4" />
                        </svg>
                      </div>
                    )}
                  </div>
                  {photoPreview && (
                    <div className="absolute inset-0 bg-black/50 rounded-full opacity-0 group-hover:opacity-100 flex items-center justify-center transition-opacity">
                      <span className="text-white text-xs">Changer</span>
                    </div>
                  )}
                </button>
                <p className="text-white/40 text-xs mt-2">
                  {photoPreview ? 'Clique pour changer' : 'Ajoute une photo (optionnel)'}
                </p>
              </div>

              {/* Pseudo */}
              <div>
                <Label htmlFor="displayName" className="text-white/60 text-sm">Ton pseudo</Label>
                <div className="relative">
                  <Input
                    id="displayName"
                    type="text"
                    data-testid="profile-displayname-input"
                    placeholder="Max 20 caractères"
                    maxLength={20}
                    value={displayName}
                    onChange={(e) => setDisplayName(e.target.value)}
                    className={`mt-1 bg-white/5 border-white/10 focus:border-white h-12 rounded-xl text-white pr-10 ${
                      pseudoAvailable === false ? 'border-red-500' : pseudoAvailable === true ? 'border-green-500' : ''
                    }`}
                  />
                  {/* Availability indicator */}
                  <div className="absolute right-3 top-1/2 -translate-y-1/2 mt-0.5">
                    {checkingPseudo ? (
                      <div className="w-5 h-5 border-2 border-white/30 border-t-white/70 rounded-full animate-spin" />
                    ) : pseudoAvailable === true ? (
                      <svg className="w-5 h-5 text-green-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                      </svg>
                    ) : pseudoAvailable === false ? (
                      <svg className="w-5 h-5 text-red-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                      </svg>
                    ) : null}
                  </div>
                </div>
                {pseudoAvailable === false && (
                  <p className="text-red-500 text-xs mt-1">Ce pseudo est déjà pris</p>
                )}
              </div>

              {/* Bio */}
              <div>
                <Label htmlFor="bio" className="text-white/60 text-sm">
                  Ta bio <span className="text-white/30">(optionnel)</span>
                </Label>
                <textarea
                  id="bio"
                  data-testid="profile-bio-input"
                  placeholder="Décris-toi en quelques mots..."
                  maxLength={150}
                  value={bio}
                  onChange={(e) => setBio(e.target.value)}
                  className="mt-1 w-full bg-white/5 border border-white/10 focus:border-white focus:outline-none p-3 rounded-xl text-white min-h-[80px] resize-none placeholder:text-white/30 text-base"
                  style={{ fontSize: '16px' }}
                />
                <p className="text-white/30 text-xs mt-1 text-right">{bio.length}/150</p>
              </div>

              {/* Country Selection */}
              <div>
                <Label className="text-white/60 text-sm">
                  Ta nationalit&eacute; <span className="text-white/30">(optionnel)</span>
                </Label>
                <div className="grid grid-cols-4 gap-2 mt-2">
                  {COUNTRY_LIST.map((c) => (
                    <button
                      key={c.code}
                      type="button"
                      onClick={() => setSelectedCountry(selectedCountry === c.code ? '' : c.code)}
                      className={`flex flex-col items-center gap-1 py-2 px-1 rounded-xl text-xs font-medium transition-all ${
                        selectedCountry === c.code
                          ? 'bg-white text-black'
                          : 'bg-white/5 text-white/70 hover:bg-white/10 border border-white/10'
                      }`}
                    >
                      <span className="text-lg">{c.flag}</span>
                      <span>{c.code}</span>
                    </button>
                  ))}
                </div>
              </div>

              {/* Divider */}
              <div className="flex items-center gap-3 py-2">
                <div className="flex-1 h-px bg-white/10"></div>
                <span className="text-white/40 text-xs uppercase tracking-wider flex items-center gap-1">
                  Tes Kinks
                  {selectedKinks.length === 0 && (
                    <span className="inline-flex items-center justify-center w-4 h-4 bg-orange-500 rounded-full text-white text-[9px] font-bold">!</span>
                  )}
                </span>
                <div className="flex-1 h-px bg-white/10"></div>
              </div>

              {/* Kinks counter */}
              <div className="flex items-center justify-between">
                <p className="text-white/50 text-sm">Quels sont tes Kinks ?</p>
                <span className="text-white font-bold text-sm">{selectedKinks.length}/10</span>
              </div>

              {/* Kinks Categories */}
              <div>
                {renderKinkCategories()}
              </div>
            </div>

            {/* Fixed bottom button */}
            <div className="mt-4 pt-4 border-t border-white/10 flex-shrink-0">
              <Button
                data-testid="profile-setup-complete-btn"
                onClick={handleProfileSetup}
                disabled={loading || uploadingPhoto || !displayName.trim() || pseudoAvailable === false}
                className="w-full h-14 rounded-xl bg-white text-black hover:bg-white/90 font-bold text-lg transition-all active:scale-[0.98] disabled:opacity-40"
              >
                {loading || uploadingPhoto ? 'Création en cours...' : 'Commencer'}
              </Button>
              
              <button
                type="button"
                onClick={() => setStep(1)}
                className="w-full text-white/40 text-sm hover:text-white/60 transition-colors mt-3"
              >
                ← Retour
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default Onboarding;
