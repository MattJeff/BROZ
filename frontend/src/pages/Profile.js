import React, { useState, useRef, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '@/contexts/AuthContext';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import axios from 'axios';
import { toast } from 'sonner';
import { getAuthToken } from '@/utils/auth';
import AccountSettings from '@/components/AccountSettings';

const BACKEND_URL = process.env.REACT_APP_BACKEND_URL;

import { KINKS_FLAT } from '@/utils/kinks';

const KINK_DATA = KINKS_FLAT;

export const Profile = () => {
  const navigate = useNavigate();
  const { user, updateUser } = useAuth();
  const fileInputRef = useRef(null);
  
  const [displayName, setDisplayName] = useState(user?.display_name || '');
  const [bio, setBio] = useState(user?.bio || '');
  const [selectedKinks, setSelectedKinks] = useState(user?.kinks || []);
  const [photoPreview, setPhotoPreview] = useState(user?.profile_photo || null);
  const [profilePhoto, setProfilePhoto] = useState(null);
  const [loading, setLoading] = useState(false);
  const [pseudoAvailable, setPseudoAvailable] = useState(null); // null = not checked, true = available, false = taken
  const [checkingPseudo, setCheckingPseudo] = useState(false);
  const [showAccountSettings, setShowAccountSettings] = useState(false);

  // Debounced pseudo availability check
  const checkPseudoAvailability = useCallback(async (pseudo) => {
    if (!pseudo || pseudo.length < 2) {
      setPseudoAvailable(null);
      return;
    }
    // Don't check if it's the same as current user's pseudo
    if (pseudo === user?.display_name) {
      setPseudoAvailable(true);
      return;
    }
    
    setCheckingPseudo(true);
    try {
      const token = getAuthToken();
      const response = await axios.get(
        `${BACKEND_URL}/api/users/check-pseudo`,
        {
          params: { name: pseudo },
          headers: { Authorization: `Bearer ${token}` }
        }
      );
      const data = response.data?.data || response.data;
      setPseudoAvailable(data.available);
    } catch (error) {
      setPseudoAvailable(null);
    } finally {
      setCheckingPseudo(false);
    }
  }, [user?.display_name]);

  // Debounce the pseudo check
  useEffect(() => {
    const timer = setTimeout(() => {
      checkPseudoAvailability(displayName);
    }, 500);
    return () => clearTimeout(timer);
  }, [displayName, checkPseudoAvailability]);

  const handlePhotoSelect = (e) => {
    const file = e.target.files[0];
    if (file) {
      if (file.size > 5 * 1024 * 1024) {
        toast.error('La photo doit faire moins de 5 Mo');
        return;
      }
      setProfilePhoto(file);
      const reader = new FileReader();
      reader.onloadend = () => setPhotoPreview(reader.result);
      reader.readAsDataURL(file);
    }
  };

  const handleKinkToggle = (kink) => {
    const idx = selectedKinks.indexOf(kink);
    if (idx !== -1) {
      setSelectedKinks(selectedKinks.filter(k => k !== kink));
    } else if (selectedKinks.length < 10) {
      setSelectedKinks([...selectedKinks, kink]);
    } else {
      toast.error('Maximum 10 kinks');
    }
  };

  const uploadPhoto = async () => {
    if (!profilePhoto) return null;
    try {
      const token = getAuthToken();
      const formData = new FormData();
      formData.append('file', profilePhoto);
      const response = await axios.post(
        BACKEND_URL + '/api/users/photo',
        formData,
        { headers: { 'Authorization': 'Bearer ' + token, 'Content-Type': 'multipart/form-data' } }
      );
      const data = response.data?.data || response.data;
      return data.photo_url;
    } catch (error) {
      toast.error('Erreur lors de l\'upload de la photo');
      return null;
    }
  };

  const handleSave = async () => {
    if (!displayName.trim()) {
      toast.error('Le pseudo est obligatoire');
      return;
    }
    if (displayName.length > 20) {
      toast.error('Le pseudo ne peut pas dépasser 20 caractères');
      return;
    }
    if (pseudoAvailable === false) {
      toast.error('Ce pseudo est déjà pris');
      return;
    }
    setLoading(true);
    try {
      const token = getAuthToken();
      let photoUrl = user?.profile_photo;
      if (profilePhoto) {
        photoUrl = await uploadPhoto();
      }
      const profileData = {
        display_name: displayName,
        bio: bio,
        kinks: selectedKinks,
        profile_photo_url: photoUrl
      };
      const response = await axios.patch(
        BACKEND_URL + '/api/users/me',
        profileData,
        { headers: { Authorization: 'Bearer ' + token } }
      );
      const updatedProfile = response.data?.data || response.data;
      updateUser({ ...user, ...updatedProfile });
      navigate(-1);
    } catch (error) {
      toast.error('Erreur lors de la sauvegarde');
    } finally {
      setLoading(false);
    }
  };

  const renderKinkCategories = () => {
    const categories = [];
    for (let i = 0; i < KINK_DATA.length; i++) {
      const category = KINK_DATA[i];
      const buttons = [];
      for (let j = 0; j < category.items.length; j++) {
        const item = category.items[j];
        const isSelected = selectedKinks.indexOf(item) !== -1;
        buttons.push(
          <button
            key={item}
            type="button"
            onClick={() => handleKinkToggle(item)}
            className={`px-3 py-1.5 rounded-full text-sm font-medium transition-all ${
              isSelected ? 'bg-white text-black shadow-lg' : 'bg-white/5 text-white/70 hover:bg-white/10 border border-white/10'
            }`}
          >
            {item}
          </button>
        );
      }
      categories.push(
        <div key={category.cat} className="mb-5">
          <h4 className="text-white/50 text-xs font-medium mb-2 uppercase tracking-wide flex items-center gap-1.5">
            <span>{category.emoji}</span>
            {category.cat}
          </h4>
          <div className="flex flex-wrap gap-2">{buttons}</div>
        </div>
      );
    }
    return categories;
  };

  return (
    <div className="min-h-screen bg-gradient-to-b from-[#0D0D0D] via-[#080808] to-[#050505] text-white">
      {/* Header */}
      <header className="sticky top-0 z-50 bg-black/80 backdrop-blur-xl border-b border-white/10">
        <div className="flex items-center justify-between p-4">
          <button onClick={() => navigate(-1)} className="text-white/70 hover:text-white">
            <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
            </svg>
          </button>
          <h1 className="text-lg font-bold">Mon profil</h1>
          <button onClick={() => setShowAccountSettings(true)} className="text-white/70 hover:text-white">
            <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="1.5">
              <path strokeLinecap="round" strokeLinejoin="round" d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.325.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 011.37.49l1.296 2.247a1.125 1.125 0 01-.26 1.431l-1.003.827c-.293.241-.438.613-.43.992a7.723 7.723 0 010 .255c-.008.378.137.75.43.991l1.004.827c.424.35.534.955.26 1.43l-1.298 2.247a1.125 1.125 0 01-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.47 6.47 0 01-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.543-.56.94-1.11.94h-2.594c-.55 0-1.019-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 01-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 01-1.369-.49l-1.297-2.247a1.125 1.125 0 01.26-1.431l1.004-.827c.292-.24.437-.613.43-.991a6.932 6.932 0 010-.255c.007-.38-.138-.751-.43-.992l-1.004-.827a1.125 1.125 0 01-.26-1.43l1.297-2.247a1.125 1.125 0 011.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.086.22-.128.332-.183.582-.495.644-.869l.214-1.28z" />
              <path strokeLinecap="round" strokeLinejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
          </button>
        </div>
      </header>

      <div className="p-4 pb-32 max-w-lg mx-auto">
        {/* Photo Upload */}
        <div className="flex flex-col items-center mb-8">
          <input
            type="file"
            ref={fileInputRef}
            onChange={handlePhotoSelect}
            accept="image/*"
            className="hidden"
          />
          <button
            type="button"
            onClick={() => fileInputRef.current.click()}
            className="relative group"
          >
            <div className={`w-28 h-28 rounded-full border-2 flex items-center justify-center overflow-hidden transition-all ${
              photoPreview ? 'border-white' : 'border-white/30 hover:border-white/50'
            }`}>
              {photoPreview ? (
                <img src={photoPreview} alt="Preview" className="w-full h-full object-cover" />
              ) : (
                <div className="text-center">
                  <svg className="w-10 h-10 mx-auto text-white/40" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M12 4v16m8-8H4" />
                  </svg>
                </div>
              )}
            </div>
            <div className="absolute inset-0 bg-black/50 rounded-full opacity-0 group-hover:opacity-100 flex items-center justify-center transition-opacity">
              <span className="text-white text-sm">{photoPreview ? 'Changer' : 'Ajouter'}</span>
            </div>
          </button>
          <p className="text-white/40 text-sm mt-2">Ta photo de profil</p>
        </div>

        {/* Pseudo */}
        <div className="mb-6">
          <Label className="text-white/60 text-sm">Pseudo</Label>
          <div className="relative">
            <Input
              type="text"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              maxLength={20}
              placeholder="Ton pseudo"
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
          <p className="text-white/30 text-xs mt-1">{displayName.length}/20 caractères</p>
        </div>

        {/* Bio */}
        <div className="mb-6">
          <Label className="text-white/60 text-sm">Bio</Label>
          <textarea
            value={bio}
            onChange={(e) => setBio(e.target.value)}
            maxLength={150}
            placeholder="Décris-toi en quelques mots..."
            className="mt-1 w-full bg-white/5 border border-white/10 focus:border-white focus:outline-none p-3 rounded-xl text-white min-h-[80px] resize-none placeholder:text-white/30"
          />
          <p className="text-white/30 text-xs mt-1 text-right">{bio.length}/150</p>
        </div>

        {/* Kinks */}
        <div className="mb-6">
          <div className="flex items-center justify-between mb-3">
            <Label className="text-white/60 text-sm">Tes Kinks</Label>
            <span className="text-white font-bold text-sm">{selectedKinks.length}/10</span>
          </div>
          <div className="max-h-[300px] overflow-y-auto pr-2">
            {renderKinkCategories()}
          </div>
        </div>
      </div>

      {/* Fixed Save Button */}
      <div className="fixed bottom-0 left-0 right-0 p-4 bg-gradient-to-t from-black via-black to-transparent">
        <div className="max-w-lg mx-auto">
          <Button
            onClick={handleSave}
            disabled={loading}
            className="w-full h-14 rounded-xl bg-white text-black hover:bg-white/90 font-bold text-lg transition-all active:scale-[0.98] disabled:opacity-40"
          >
            {loading ? 'Sauvegarde...' : 'Enregistrer'}
          </Button>
        </div>
      </div>
      <AccountSettings isOpen={showAccountSettings} onClose={() => setShowAccountSettings(false)} />
    </div>
  );
};

export default Profile;
