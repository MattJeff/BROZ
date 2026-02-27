import React from 'react';
import { useNavigate } from 'react-router-dom';

export const SafetyScreen = () => {
  const navigate = useNavigate();

  const handleContinue = () => {
    sessionStorage.setItem('brozr_safety_seen', 'true');
    navigate('/video-call');
  };

  const safetyPoints = [
    {
      icon: (
        <div className="w-12 h-12 rounded-full bg-red-500/20 flex items-center justify-center flex-shrink-0">
          <svg className="w-6 h-6 text-red-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path strokeLinecap="round" strokeLinejoin="round" d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636" />
          </svg>
        </div>
      ),
      title: "Comportement illégal strictement interdit",
      description: "Suspension immédiate et signalement aux autorités"
    },
    {
      icon: (
        <div className="w-12 h-12 rounded-full bg-orange-500/20 flex items-center justify-center flex-shrink-0">
          <svg className="w-6 h-6 text-orange-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path strokeLinecap="round" strokeLinejoin="round" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
            <path strokeLinecap="round" strokeLinejoin="round" d="M3 3l18 18" />
          </svg>
        </div>
      ),
      title: "Enregistrement d'écran",
      description: "Sans consentement : strictement interdit"
    },
    {
      icon: (
        <div className="w-12 h-12 rounded-full bg-blue-500/20 flex items-center justify-center flex-shrink-0">
          <svg className="w-6 h-6 text-blue-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path strokeLinecap="round" strokeLinejoin="round" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
        </div>
      ),
      title: "Assistance 24h/24 7j/7",
      description: "Les rapports sont examinés en continu."
    },
    {
      icon: (
        <div className="w-12 h-12 rounded-full bg-red-400/20 flex items-center justify-center flex-shrink-0">
          <svg className="w-6 h-6 text-red-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="12" cy="12" r="9" />
            <path strokeLinecap="round" strokeLinejoin="round" d="M8 12h8" />
          </svg>
        </div>
      ),
      title: "18+ uniquement",
      description: "Non accessible aux mineurs"
    }
  ];

  return (
    <div className="fixed inset-0 bg-gradient-to-b from-[#0D0D0D] via-[#080808] to-[#050505] flex items-center justify-center px-6">
      <div className="flex flex-col max-w-md w-full h-full max-h-[700px] py-10 justify-between">
        {/* Top section with title */}
        <div className="flex-shrink-0">
          <h1 className="text-3xl font-black text-white leading-tight">
            Restez sûr et<br />amusez-vous
          </h1>
        </div>

        {/* Middle section with safety points - spread out */}
        <div className="flex-1 flex flex-col justify-center py-8">
          <div className="space-y-6">
            {safetyPoints.map((point, index) => (
              <div key={index} className="flex items-start gap-4">
                {point.icon}
                <div className="flex-1 min-w-0">
                  <h3 className="font-bold text-white text-base">{point.title}</h3>
                  <p className="text-white/50 text-sm mt-1">{point.description}</p>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Bottom section with CTA */}
        <div className="flex-shrink-0">
          <button
            onClick={handleContinue}
            className="w-full py-4 bg-white text-black font-black text-lg rounded-full transition-all active:scale-[0.98] hover:bg-white/90"
            data-testid="safety-continue-btn"
          >
            COMPRIS
          </button>

          <a 
            href="#" 
            className="block text-center mt-4 text-white/40 underline text-sm"
            onClick={(e) => e.preventDefault()}
          >
            Règles de la communauté Brozr
          </a>
        </div>
      </div>
    </div>
  );
};

export default SafetyScreen;
