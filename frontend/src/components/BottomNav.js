import React from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { Video, MessageCircle, User } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export const BottomNav = () => {
  const navigate = useNavigate();
  const location = useLocation();
  const { t } = useTranslation();

  const navItems = [
    { path: '/live', icon: Video, label: t('nav.live'), testId: 'nav-live' },
    { path: '/space', icon: MessageCircle, label: t('nav.space'), testId: 'nav-space' },
    { path: '/profile', icon: User, label: t('nav.profile'), testId: 'nav-profile' }
  ];

  return (
    <div className="fixed bottom-0 left-0 right-0 h-16 bg-black/80 backdrop-blur-lg border-t border-white/10 flex justify-around items-center z-50">
      {navItems.map((item) => {
        const Icon = item.icon;
        const isActive = location.pathname === item.path;
        return (
          <button
            key={item.path}
            data-testid={item.testId}
            onClick={() => navigate(item.path)}
            className={`flex flex-col items-center justify-center px-4 py-2 transition-colors ${
              isActive ? 'text-blue-500' : 'text-white/60 hover:text-white'
            }`}
          >
            <Icon className="w-6 h-6 mb-1" />
            <span className="text-xs">{item.label}</span>
          </button>
        );
      })}
    </div>
  );
};

export default BottomNav;
