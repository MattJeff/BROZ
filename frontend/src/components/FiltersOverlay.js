import React, { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { X, MapPin, User, Users, Globe } from 'lucide-react';
import { useAuth } from '@/contexts/AuthContext';
import { useNavigate } from 'react-router-dom';
import axios from 'axios';
import { toast } from 'sonner';
import { getAuthToken } from '@/utils/auth';

const BACKEND_URL = process.env.REACT_APP_BACKEND_URL;

const COUNTRIES = [
  { code: 'FR', name: 'France' },
  { code: 'US', name: 'United States' },
  { code: 'GB', name: 'United Kingdom' },
  { code: 'DE', name: 'Germany' },
  { code: 'ES', name: 'Spain' },
  { code: 'IT', name: 'Italy' }
];

const KINKS = [
  'Muscular', 'Twink', 'Bear', 'Daddy', 'Athletic', 'Smooth', 'Hairy',
  'Tattooed', 'Pierced', 'Fetish', 'Leather', 'Roleplay', 'Dominant',
  'Submissive', 'Versatile', 'Casual', 'Intimate', 'Exhibitionist', 'Voyeur'
];

export const FiltersOverlay = ({ onClose, currentFilters, onUpdateFilters }) => {
  const { t } = useTranslation();
  const { user } = useAuth();
  const navigate = useNavigate();
  const [filters, setFilters] = useState(currentFilters || {
    country: null,
    age_min: 18,
    age_max: 70,
    kinks: [],
    max_distance_km: null
  });

  const isPremium = user?.is_premium;

  const handleSave = async () => {
    if (!isPremium && (filters.age_min !== 18 || filters.age_max !== 70 || filters.kinks.length > 0 || filters.max_distance_km)) {
      toast.error('Premium required for advanced filters');
      return;
    }

    try {
      const token = getAuthToken();
      await axios.post(
        `${BACKEND_URL}/api/users/filter-preferences`,
        filters,
        { headers: { Authorization: `Bearer ${token}` }, withCredentials: true }
      );
      onUpdateFilters(filters);
      toast.success('Filters updated!');
      onClose();
    } catch (error) {
      console.error('Filter save error:', error);
      toast.error('Failed to save filters');
    }
  };

  const handleKinkToggle = (kink) => {
    if (!isPremium) {
      navigate('/premium');
      return;
    }
    if (filters.kinks.includes(kink)) {
      setFilters({ ...filters, kinks: filters.kinks.filter(k => k !== kink) });
    } else if (filters.kinks.length < 5) {
      setFilters({ ...filters, kinks: [...filters.kinks, kink] });
    } else {
      toast.error('Maximum 5 kinks');
    }
  };

  return (
    <div className="fixed inset-0 bg-black/90 backdrop-blur-sm z-50 flex items-end md:items-center justify-center">
      <div className="bg-[#0A0A0A] border-t md:border border-white/10 rounded-t-3xl md:rounded-2xl w-full md:max-w-2xl max-h-[80vh] overflow-y-auto" data-testid="filters-overlay">
        {/* Header */}
        <div className="sticky top-0 bg-[#0A0A0A] border-b border-white/10 p-4 flex items-center justify-between">
          <h2 className="text-xl font-bold" style={{ fontFamily: 'Unbounded, sans-serif' }}>
            {t('live.filters')}
          </h2>
          <Button
            data-testid="close-filters-btn"
            onClick={onClose}
            size="icon"
            className="rounded-full bg-white/5 hover:bg-white/10"
          >
            <X className="w-5 h-5" />
          </Button>
        </div>

        {/* Filters */}
        <div className="p-6 space-y-6">
          {/* Country Filter (FREE) */}
          <div className="space-y-3">
            <div className="flex items-center gap-2">
              <Globe className="w-5 h-5 text-white/60" />
              <h3 className="font-bold">Country <span className="text-xs text-white/40">(Free)</span></h3>
            </div>
            <select
              data-testid="country-filter"
              value={filters.country || ''}
              onChange={(e) => setFilters({ ...filters, country: e.target.value || null })}
              className="w-full bg-white/5 border border-white/10 rounded-lg p-3 text-white focus:outline-none focus:border-white/20"
            >
              <option value="">No preference</option>
              {COUNTRIES.map(c => <option key={c.code} value={c.code}>{c.name}</option>)}
            </select>
          </div>

          {/* Age Filter (PREMIUM) */}
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <User className="w-5 h-5 text-white/60" />
                <h3 className="font-bold">Age Range {!isPremium && <span className="text-xs text-amber-500">(Premium)</span>}</h3>
              </div>
              {!isPremium && (
                <Button
                  data-testid="premium-cta-age"
                  onClick={() => navigate('/premium')}
                  className="text-xs h-7 px-3 rounded-full bg-white text-black hover:bg-white/90"
                >
                  Upgrade
                </Button>
              )}
            </div>
            <div className="flex items-center gap-4">
              <input
                type="range"
                min="18"
                max="70"
                value={filters.age_min}
                onChange={(e) => isPremium && setFilters({ ...filters, age_min: parseInt(e.target.value) })}
                disabled={!isPremium}
                className="flex-1"
                data-testid="age-min-slider"
              />
              <span className="text-sm text-white/60 w-12">{filters.age_min}</span>
            </div>
            <div className="flex items-center gap-4">
              <input
                type="range"
                min="18"
                max="70"
                value={filters.age_max}
                onChange={(e) => isPremium && setFilters({ ...filters, age_max: parseInt(e.target.value) })}
                disabled={!isPremium}
                className="flex-1"
                data-testid="age-max-slider"
              />
              <span className="text-sm text-white/60 w-12">{filters.age_max}</span>
            </div>
          </div>

          {/* Distance Filter (PREMIUM) */}
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <MapPin className="w-5 h-5 text-white/60" />
                <h3 className="font-bold">Max Distance {!isPremium && <span className="text-xs text-amber-500">(Premium)</span>}</h3>
              </div>
              {!isPremium && (
                <Button
                  data-testid="premium-cta-distance"
                  onClick={() => navigate('/premium')}
                  className="text-xs h-7 px-3 rounded-full bg-white text-black hover:bg-white/90"
                >
                  Upgrade
                </Button>
              )}
            </div>
            <div className="flex items-center gap-4">
              <input
                type="range"
                min="1"
                max="700"
                value={filters.max_distance_km || 700}
                onChange={(e) => isPremium && setFilters({ ...filters, max_distance_km: parseInt(e.target.value) })}
                disabled={!isPremium}
                className="flex-1"
                data-testid="distance-slider"
              />
              <span className="text-sm text-white/60 w-16">{filters.max_distance_km || 'No limit'} km</span>
            </div>
          </div>

          {/* Kinks Filter (PREMIUM) */}
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Users className="w-5 h-5 text-white/60" />
                <h3 className="font-bold">Kinks ({filters.kinks.length}/5) {!isPremium && <span className="text-xs text-amber-500">(Premium)</span>}</h3>
              </div>
              {!isPremium && (
                <Button
                  data-testid="premium-cta-kinks"
                  onClick={() => navigate('/premium')}
                  className="text-xs h-7 px-3 rounded-full bg-white text-black hover:bg-white/90"
                >
                  Upgrade
                </Button>
              )}
            </div>
            <div className="grid grid-cols-2 md:grid-cols-3 gap-2">
              {KINKS.map(kink => (
                <button
                  key={kink}
                  data-testid={`kink-filter-${kink.toLowerCase()}`}
                  onClick={() => handleKinkToggle(kink)}
                  className={`p-2 rounded-lg text-sm transition-all ${
                    filters.kinks.includes(kink)
                      ? 'bg-white text-black'
                      : 'bg-white/5 text-white/60 hover:bg-white/10'
                  } ${!isPremium ? 'opacity-50 cursor-not-allowed' : ''}`}
                  disabled={!isPremium}
                >
                  {kink}
                </button>
              ))}
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="sticky bottom-0 bg-[#0A0A0A] border-t border-white/10 p-4">
          <Button
            data-testid="save-filters-btn"
            onClick={handleSave}
            className="w-full h-12 rounded-lg bg-white text-black hover:bg-white/90 font-bold"
          >
            Apply Filters
          </Button>
        </div>
      </div>
    </div>
  );
};

export default FiltersOverlay;
