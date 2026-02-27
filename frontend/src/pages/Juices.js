import React from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '@/contexts/AuthContext';
import { Button } from '@/components/ui/button';
import { Sparkles } from 'lucide-react';
import { toast } from 'sonner';

export const Juices = () => {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { user } = useAuth();

  const juicePacks = [
    { id: 'small', amount: 100, price: '4.99', bonus: 0 },
    { id: 'medium', amount: 250, price: '9.99', bonus: 25, recommended: true },
    { id: 'large', amount: 600, price: '19.99', bonus: 100 },
    { id: 'xlarge', amount: 1500, price: '49.99', bonus: 300 },
    { id: 'mega', amount: 5000, price: '99.99', bonus: 1500 }
  ];

  const handlePurchase = (pack) => {
    toast.success(`Purchased ${pack.amount} Juice! (Simulated)`);
    setTimeout(() => navigate('/profile'), 1500);
  };

  return (
    <div
      className="min-h-screen bg-[#050505] text-white p-6"
      style={{
        backgroundImage: 'url(https://images.unsplash.com/photo-1706554596955-de04dd60e540?crop=entropy&cs=srgb&fm=jpg&ixid=M3w3NTY2NzF8MHwxfHNlYXJjaHwxfHxhYnN0cmFjdCUyMG5lb24lMjBibHVlJTIwcHVycGxlJTIwZmx1aWQlMjAzZHxlbnwwfHx8fDE3Njk4ODEwMzZ8MA&ixlib=rb-4.1.0&q=85)',
        backgroundSize: 'cover',
        backgroundPosition: 'center'
      }}
    >
      <div className="absolute inset-0 bg-black/80"></div>
      
      <div className="relative z-10 container mx-auto max-w-2xl">
        <Button
          data-testid="juice-back-btn"
          onClick={() => navigate(-1)}
          className="mb-6 bg-transparent hover:bg-white/10 border border-white/20 rounded-full"
        >
          ← Back
        </Button>

        <div className="text-center mb-12">
          <Sparkles className="w-16 h-16 text-cyan-400 mx-auto mb-4" />
          <h1
            className="text-4xl font-black mb-4"
            style={{ fontFamily: 'Unbounded, sans-serif' }}
            data-testid="juice-title"
          >
            {t('juice.title')}
          </h1>
          <div className="inline-block bg-black/60 backdrop-blur-xl border border-cyan-500/30 rounded-full px-6 py-3">
            <p className="text-sm text-white/60">Your Balance</p>
            <p className="text-3xl font-black text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-purple-600" data-testid="current-juice-balance">
              {user?.juice_balance || 0} Juice
            </p>
          </div>
        </div>

        {/* Juice Packs */}
        <div className="space-y-4" data-testid="juice-packs">
          {juicePacks.map((pack) => (
            <div
              key={pack.id}
              className={`bg-black/60 backdrop-blur-xl border rounded-2xl p-6 relative hover:border-cyan-500/50 transition-all ${
                pack.recommended
                  ? 'border-cyan-500 shadow-[0_0_20px_rgba(34,211,238,0.3)]'
                  : 'border-white/10'
              }`}
            >
              {pack.recommended && (
                <div className="absolute -top-3 left-1/2 transform -translate-x-1/2 bg-gradient-to-r from-cyan-500 to-purple-600 text-white text-xs font-bold px-4 py-1 rounded-full">
                  BEST VALUE
                </div>
              )}
              {pack.bonus > 0 && (
                <div className="absolute -top-3 right-4 bg-purple-500 text-white text-xs font-bold px-4 py-1 rounded-full">
                  +{pack.bonus} BONUS
                </div>
              )}
              <div className="flex items-center justify-between mb-4">
                <div className="flex items-center gap-3">
                  <Sparkles className="w-8 h-8 text-cyan-400" />
                  <div>
                    <h3 className="text-2xl font-black">
                      {pack.amount}
                      {pack.bonus > 0 && (
                        <span className="text-purple-400 text-lg"> +{pack.bonus}</span>
                      )}
                    </h3>
                    <p className="text-sm text-white/60">Juice</p>
                  </div>
                </div>
                <div className="text-right">
                  <p className="text-3xl font-black">€{pack.price}</p>
                </div>
              </div>
              <Button
                data-testid={`buy-juice-btn-${pack.id}`}
                onClick={() => handlePurchase(pack)}
                className={`w-full h-12 rounded-full font-bold transition-all duration-300 active:scale-95 ${
                  pack.recommended
                    ? 'bg-gradient-to-r from-cyan-500 to-purple-600 hover:from-cyan-600 hover:to-purple-700 shadow-[0_0_20px_rgba(34,211,238,0.4)]'
                    : 'bg-white/10 hover:bg-white/20'
                }`}
              >
                {t('juice.buy_juice')}
              </Button>
            </div>
          ))}
        </div>

        <p className="text-center text-xs text-white/40 mt-6">
          Payment is simulated. No real charges will be made.
        </p>
      </div>
    </div>
  );
};

export default Juices;
