import React, { useState, useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { Button } from '@/components/ui/button';

const fontUrls = [
  'https://fonts.googleapis.com/css2?family=Outfit:wght@400;500;600;700;800&family=Poppins:wght@600;700;800&display=swap',
  'https://api.fontshare.com/v2/css?f[]=satoshi@500,700,900&display=swap',
];

const heroStyles = `
  :root {
    --brozr-chrome-hi: #E8E8EE;
    --brozr-chrome-mid: #9A9AAE;
    --brozr-chrome-lo: #6A6A7E;
    --brozr-chrome-dim: #4A4A5E;
    --brozr-bg: #060608;
    --brozr-surface: #0A0A0C;
    --brozr-border: rgba(180,185,200,0.08);
    --brozr-border-hover: rgba(180,185,200,0.16);
    --brozr-text-body: rgba(200,205,220,0.6);
    --brozr-emerald: #34d399;
    --brozr-emerald-glow: rgba(52,211,153,0.25);
  }

  .brozr-noise::before {
    content: '';
    position: absolute;
    inset: 0;
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)' opacity='0.025'/%3E%3C/svg%3E");
    background-repeat: repeat;
    pointer-events: none;
    z-index: 1;
  }

  @keyframes brozr-fade-up {
    from { opacity: 0; transform: translateY(24px); }
    to { opacity: 1; transform: translateY(0); }
  }
  @keyframes brozr-scale-in {
    from { opacity: 0; transform: scale(0.92); }
    to { opacity: 1; transform: scale(1); }
  }
  @keyframes brozr-float {
    0%, 100% { transform: translateY(0px); }
    50% { transform: translateY(-6px); }
  }
  @keyframes brozr-chrome-shimmer {
    0% { background-position: -200% center; }
    100% { background-position: 200% center; }
  }

  .brozr-stagger-1 { animation: brozr-fade-up 0.7s cubic-bezier(0.16,1,0.3,1) 0.1s both; }
  .brozr-stagger-2 { animation: brozr-fade-up 0.7s cubic-bezier(0.16,1,0.3,1) 0.25s both; }
  .brozr-stagger-3 { animation: brozr-scale-in 0.8s cubic-bezier(0.16,1,0.3,1) 0.4s both; }
  .brozr-stagger-4 { animation: brozr-fade-up 0.6s cubic-bezier(0.16,1,0.3,1) 0.6s both; }
  .brozr-stagger-5 { animation: brozr-fade-up 0.6s cubic-bezier(0.16,1,0.3,1) 0.75s both; }

  .brozr-reveal {
    opacity: 0;
    transform: translateY(32px);
    transition: opacity 0.7s cubic-bezier(0.16,1,0.3,1), transform 0.7s cubic-bezier(0.16,1,0.3,1);
  }
  .brozr-reveal.visible {
    opacity: 1;
    transform: translateY(0);
  }

  .brozr-chrome-btn {
    background: linear-gradient(135deg, var(--brozr-chrome-hi) 0%, var(--brozr-chrome-mid) 40%, var(--brozr-chrome-hi) 80%, var(--brozr-chrome-mid) 100%);
    background-size: 200% 100%;
    animation: brozr-chrome-shimmer 4s ease-in-out infinite;
    color: var(--brozr-bg);
  }

  .brozr-glow-btn {
    position: relative;
    overflow: hidden;
  }
  .brozr-glow-btn::after {
    content: '';
    position: absolute;
    inset: -2px;
    background: linear-gradient(135deg, rgba(200,205,220,0.3), rgba(150,155,170,0.1));
    border-radius: inherit;
    z-index: -1;
    opacity: 0;
    transition: opacity 0.3s;
    filter: blur(14px);
  }
  .brozr-glow-btn:hover::after {
    opacity: 1;
  }

  .brozr-feature-row {
    transition: transform 0.4s cubic-bezier(0.16,1,0.3,1);
  }
  .brozr-feature-row:hover {
    transform: scale(1.01);
  }
`;

const useScrollReveal = () => {
  const ref = useRef(null);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          el.classList.add('visible');
          observer.unobserve(el);
        }
      },
      { threshold: 0.15 }
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, []);
  return ref;
};

const Reveal = ({ children, className = '' }) => {
  const ref = useScrollReveal();
  return (
    <div ref={ref} className={`brozr-reveal ${className}`}>
      {children}
    </div>
  );
};

const ChromeAccent = ({ children }) => (
  <span
    style={{
      background: 'linear-gradient(135deg, #E8E8EE, #9A9AAE)',
      WebkitBackgroundClip: 'text',
      WebkitTextFillColor: 'transparent',
    }}
  >
    {children}
  </span>
);

export const Hero = () => {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [scrolled, setScrolled] = useState(false);
  const [onlineCount, setOnlineCount] = useState(1230);
  const [pageReady, setPageReady] = useState(false);
  const videoRef = useRef(null);
  const videoRef2 = useRef(null);

  // Load fonts via <link> in <head> (non-blocking, avoids @import in <style>)
  useEffect(() => {
    fontUrls.forEach(url => {
      if (!document.querySelector(`link[href="${url}"]`)) {
        const link = document.createElement('link');
        link.rel = 'stylesheet';
        link.href = url;
        document.head.appendChild(link);
      }
    });
  }, []);

  useEffect(() => {
    const handleScroll = () => setScrolled(window.scrollY > 20);
    window.addEventListener('scroll', handleScroll);
    return () => window.removeEventListener('scroll', handleScroll);
  }, []);

  useEffect(() => {
    const video = videoRef.current;
    if (!video) { setPageReady(true); return; }
    if (video.readyState >= 3) { setPageReady(true); return; }
    const onReady = () => setPageReady(true);
    video.addEventListener('canplay', onReady, { once: true });
    video.addEventListener('loadeddata', onReady, { once: true });
    const timeout = setTimeout(() => setPageReady(true), 1200);
    return () => { video.removeEventListener('canplay', onReady); video.removeEventListener('loadeddata', onReady); clearTimeout(timeout); };
  }, []);

  useEffect(() => {
    const videos = [videoRef.current, videoRef2.current].filter(Boolean);
    const tryPlay = () => {
      videos.forEach(video => { if (video.paused) video.play().catch(() => {}); });
    };
    tryPlay();
    const handlers = [];
    videos.forEach(video => {
      const handle = () => video.play().catch(() => {});
      video.addEventListener('canplay', handle);
      video.addEventListener('loadeddata', handle);
      handlers.push({ video, handle });
    });
    const handleInteraction = () => {
      tryPlay();
      document.removeEventListener('touchstart', handleInteraction);
      document.removeEventListener('click', handleInteraction);
    };
    document.addEventListener('touchstart', handleInteraction, { once: true, passive: true });
    document.addEventListener('click', handleInteraction, { once: true });
    let observer;
    if ('IntersectionObserver' in window) {
      observer = new IntersectionObserver((entries) => {
        entries.forEach(entry => { if (entry.isIntersecting && entry.target.paused) entry.target.play().catch(() => {}); });
      }, { threshold: 0.25 });
      videos.forEach(video => observer.observe(video));
    }
    return () => {
      handlers.forEach(({ video, handle }) => { video.removeEventListener('canplay', handle); video.removeEventListener('loadeddata', handle); });
      document.removeEventListener('touchstart', handleInteraction);
      document.removeEventListener('click', handleInteraction);
      if (observer) observer.disconnect();
    };
  }, []);

  useEffect(() => {
    const interval = setInterval(() => {
      setOnlineCount(prev => {
        const change = Math.floor(Math.random() * 21) - 10;
        return Math.max(1180, Math.min(1350, prev + change));
      });
    }, 5000);
    return () => clearInterval(interval);
  }, []);

  const handleGoogleAuth = () => {
    const redirectUrl = window.location.origin + '/auth/callback';
    window.location.href = `https://auth.emergentagent.com/?redirect=${encodeURIComponent(redirectUrl)}`;
  };

  const fontPoppins = { fontFamily: "'Poppins', sans-serif" };
  const fontSatoshi = { fontFamily: "'Satoshi', sans-serif" };
  const fontOutfit = { fontFamily: "'Outfit', sans-serif" };

  return (
    <>
      <style>{heroStyles}</style>
      <div
        className={`min-h-screen text-white overflow-x-hidden transition-opacity duration-500 ${pageReady ? 'opacity-100' : 'opacity-0'}`}
        style={{ ...fontOutfit, background: 'linear-gradient(180deg, #060608 0%, #08080C 30%, #060608 60%, #0A0A0E 100%)' }}
      >
        {/* Ambient orbs */}
        <div className="fixed inset-0 pointer-events-none overflow-hidden z-0">
          <div className="absolute top-[-25%] left-[-15%] w-[700px] h-[700px] rounded-full" style={{ background: 'radial-gradient(circle, rgba(200,205,220,0.035) 0%, transparent 70%)' }} />
          <div className="absolute bottom-[-15%] right-[-10%] w-[500px] h-[500px] rounded-full" style={{ background: 'radial-gradient(circle, rgba(180,185,200,0.025) 0%, transparent 70%)' }} />
        </div>
        <div className="fixed inset-0 brozr-noise pointer-events-none z-[1]" />

        {/* ─── HEADER ─── */}
        <header className={`fixed top-0 left-0 right-0 z-50 transition-all duration-500 ${
          scrolled
            ? 'bg-[#060608]/88 backdrop-blur-2xl border-b border-[rgba(180,185,200,0.08)] shadow-[0_4px_30px_rgba(0,0,0,0.4)]'
            : 'bg-transparent'
        }`}>
          <div className="container mx-auto px-4 sm:px-8 lg:px-12 h-16 flex items-center justify-between">
            <div className={`hidden sm:flex items-center gap-3 transition-all duration-500 ${scrolled ? 'opacity-0 translate-y-[-4px]' : 'opacity-100 translate-y-0'}`}>
              <h2
                className="text-3xl lg:text-4xl font-bold leading-none text-white"
                style={{ ...fontPoppins, fontWeight: 700, letterSpacing: '-0.02em' }}
              >
                Brozr
              </h2>
              <div className="w-px h-7 bg-gradient-to-b from-transparent via-[rgba(180,185,200,0.15)] to-transparent" />
              <p
                className="text-[10px] lg:text-[11px] font-medium leading-tight tracking-wide uppercase"
                style={{ ...fontPoppins, color: '#4A4A5E' }}
              >
                Real Guy<br />Real Fun
              </p>
            </div>
            <div className="sm:hidden" />
            <div className="flex items-center gap-2.5">
              <Button
                data-testid="header-login-btn"
                onClick={() => navigate('/login')}
                className="h-9 px-5 text-sm rounded-lg bg-[rgba(180,185,200,0.06)] hover:bg-[rgba(180,185,200,0.1)] text-[#9A9AAE] hover:text-[#E8E8EE] border border-[rgba(180,185,200,0.08)] hover:border-[rgba(180,185,200,0.16)] font-medium transition-all duration-300"
                style={fontOutfit}
              >
                Se connecter
              </Button>
              <Button
                data-testid="header-signup-btn"
                onClick={() => navigate('/signup')}
                className="brozr-chrome-btn h-9 px-5 text-sm rounded-lg font-bold transition-all duration-300 shadow-[0_0_20px_rgba(180,185,200,0.08)]"
                style={fontOutfit}
              >
                S'inscrire
              </Button>
            </div>
          </div>
        </header>

        {/* ─── HERO SECTION ─── */}
        <section className="flex flex-col pt-14 sm:pt-16 pb-2 sm:pb-6 px-4 sm:px-6 relative z-10">
          <div className="container mx-auto max-w-5xl flex flex-col gap-3 sm:gap-3">

            {/* Mobile logo — smaller tagline */}
            <div className="flex items-center justify-center gap-2.5 sm:hidden brozr-stagger-1">
              <h2
                className="text-5xl font-bold leading-none text-white"
                style={{ ...fontPoppins, fontWeight: 700, letterSpacing: '-0.02em' }}
                data-testid="hero-main-title"
              >
                Brozr
              </h2>
              <div className="w-px h-7 bg-gradient-to-b from-transparent via-[rgba(180,185,200,0.15)] to-transparent" />
              <p
                className="text-[9px] font-medium leading-tight tracking-wide uppercase"
                style={{ ...fontPoppins, color: '#4A4A5E' }}
              >
                Real Guy<br />Real Fun
              </p>
            </div>

            {/* Headline + Video */}
            <div className="flex flex-col items-center justify-center gap-1.5 sm:gap-3 mt-0 sm:mt-1">
              <div className="text-center brozr-stagger-2">
                <p
                  className="text-2xl sm:text-2xl lg:text-3xl font-black leading-snug tracking-tight"
                  style={{ ...fontSatoshi, color: '#E8E8EE' }}
                >
                  Vibrer. Connecter. <ChromeAccent>Monétiser</ChromeAccent>.
                </p>
                <p
                  className="text-base sm:text-base lg:text-lg leading-relaxed mt-2 max-w-lg mx-auto"
                  style={{ ...fontOutfit, color: 'rgba(255,255,255,0.88)' }}
                >
                  Live cam instantané avec filtres.
                  <br /> Suis les mecs qui te font vibrer, retrouve-les
                  <br /> en un clic et envoie ou reçois des&nbsp;tips
                </p>
              </div>

              {/* Video Preview */}
              <div className="relative flex items-center justify-center w-full h-[290px] sm:h-[230px] lg:h-[250px] brozr-stagger-3">
                <div
                  className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 w-[180px] h-[280px] sm:w-[160px] sm:h-[260px] rounded-3xl"
                  style={{ background: 'radial-gradient(circle, rgba(180,185,200,0.06) 0%, transparent 70%)' }}
                />
                <div className="absolute left-1/2 -translate-x-[130%] sm:-translate-x-[140%] w-[105px] sm:w-[95px] lg:w-[105px] -rotate-6 opacity-60 z-0">
                  <video
                    ref={videoRef2}
                    src="/Brozr_Preview_UI3.mp4"
                    autoPlay loop muted playsInline
                    webkit-playsinline="true"
                    preload="auto"
                    className="w-full h-auto rounded-2xl shadow-xl ring-1 ring-[rgba(180,185,200,0.06)]"
                  />
                </div>
                <div className="relative w-[142px] sm:w-[125px] lg:w-[135px] z-10">
                  <video
                    ref={videoRef}
                    src="/Brozr_Preview_UI1.mp4"
                    autoPlay loop muted playsInline
                    webkit-playsinline="true"
                    preload="auto"
                    className="w-full h-auto rounded-2xl shadow-2xl ring-1 ring-[rgba(180,185,200,0.1)]"
                  />
                </div>
                <div className="absolute left-1/2 translate-x-[30%] sm:translate-x-[40%] w-[105px] sm:w-[95px] lg:w-[105px] rotate-6 opacity-60 z-0">
                  <img
                    src="/UIvf2.png"
                    alt="Communauté"
                    className="w-full h-auto rounded-2xl shadow-xl ring-1 ring-[rgba(180,185,200,0.06)]"
                  />
                </div>
              </div>
            </div>

            {/* Online Badge — GREEN */}
            <div className="flex justify-center brozr-stagger-4" data-testid="online-badge">
              <div className="inline-flex items-center gap-2 px-4 py-2 rounded-full bg-[rgba(52,211,153,0.06)] border border-[rgba(52,211,153,0.15)] backdrop-blur-md">
                <span className="relative flex h-2 w-2">
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" />
                  <span className="relative inline-flex rounded-full h-2 w-2 bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.25)]" />
                </span>
                <span className="text-xs font-medium text-[rgba(52,211,153,0.7)]" style={fontOutfit}>
                  <span className="text-emerald-400 font-bold tabular-nums">{onlineCount.toLocaleString()}</span> bros en ligne
                </span>
              </div>
            </div>

            {/* CTAs */}
            <div className="text-center space-y-2 sm:space-y-3 brozr-stagger-5">
              <div className="flex flex-row gap-2.5 sm:gap-3 justify-center px-2 sm:px-0">
                <Button
                  data-testid="hero-login-btn"
                  onClick={() => navigate('/login')}
                  className="h-11 sm:h-12 flex-1 sm:flex-none sm:min-w-[200px] rounded-xl text-sm font-semibold bg-[rgba(180,185,200,0.06)] text-[#9A9AAE] hover:text-[#E8E8EE] hover:bg-[rgba(180,185,200,0.1)] border border-[rgba(180,185,200,0.08)] hover:border-[rgba(180,185,200,0.16)] transition-all duration-300"
                  style={fontOutfit}
                >
                  Se connecter
                </Button>
                <Button
                  data-testid="hero-signup-btn"
                  onClick={() => navigate('/signup')}
                  className="brozr-chrome-btn brozr-glow-btn h-11 sm:h-12 flex-1 sm:flex-none sm:min-w-[200px] rounded-xl text-sm font-bold hover:scale-[1.03] transition-all duration-300 shadow-[0_0_25px_rgba(180,185,200,0.08)]"
                  style={fontOutfit}
                >
                  S'inscrire
                </Button>
              </div>

              {/* Trust — visible grey */}
              <p className="sm:hidden text-[rgba(200,205,220,0.5)] text-[10px]" style={fontOutfit}>
                Inscription gratuite · Commence en 30 secondes · Connexion sécurisée
              </p>
              <div className="hidden sm:flex flex-wrap justify-center items-center gap-x-4 text-[rgba(200,205,220,0.55)] text-xs" style={fontOutfit}>
                {[
                  { icon: 'M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z', label: 'Inscription gratuite' },
                  { icon: 'M10 18a8 8 0 100-16 8 8 0 000 16zm1-12a1 1 0 10-2 0v4a1 1 0 00.293.707l2.828 2.829a1 1 0 101.415-1.415L11 9.586V6z', label: 'Commence en 30 secondes' },
                  { icon: 'M5 9V7a5 5 0 0110 0v2a2 2 0 012 2v5a2 2 0 01-2 2H5a2 2 0 01-2-2v-5a2 2 0 012-2zm8-2v2H7V7a3 3 0 016 0z', label: 'Connexion sécurisée' },
                ].map((item, i) => (
                  <React.Fragment key={i}>
                    {i > 0 && <span className="text-[rgba(180,185,200,0.08)]">·</span>}
                    <div className="flex items-center gap-1.5">
                      <svg className="w-3 h-3" style={{ fill: 'rgba(200,205,220,0.45)' }} viewBox="0 0 20 20">
                        <path fillRule="evenodd" d={item.icon} clipRule="evenodd" />
                      </svg>
                      <span>{item.label}</span>
                    </div>
                  </React.Fragment>
                ))}
              </div>
            </div>
          </div>
        </section>

        {/* ─── FEATURE: Match instantané ─── */}
        <Reveal>
          <section className="py-6 sm:py-12 lg:py-16 px-4 sm:px-6 relative z-10">
            <div className="container mx-auto max-w-5xl">
              <div className="brozr-feature-row flex items-center justify-center gap-6 sm:gap-10 p-4 sm:p-6 rounded-2xl hover:bg-[rgba(180,185,200,0.02)] transition-colors duration-500">
                <div className="text-left max-w-[260px] sm:max-w-[340px]">
                  <p className="text-lg sm:text-2xl lg:text-3xl font-black leading-snug tracking-tight" style={{ ...fontSatoshi, color: '#E8E8EE' }}>
                    Match instantané, sans <ChromeAccent>attendre</ChromeAccent>
                  </p>
                  <p className="text-xs sm:text-sm lg:text-base leading-relaxed mt-2" style={{ ...fontOutfit, color: 'rgba(200,205,220,0.6)' }}>
                    Match instantanément et choisis avec qui vibrer grâce aux filtres par âge, localisation et kinks
                  </p>
                </div>
                <div className="w-[95px] sm:w-[125px] lg:w-[155px] flex-shrink-0 transform rotate-2">
                  <video
                    src="/Brozr_Preview_UI1.mp4"
                    autoPlay loop muted playsInline
                    webkit-playsinline="true"
                    preload="auto"
                    className="w-full h-auto rounded-2xl shadow-2xl ring-1 ring-[rgba(180,185,200,0.08)]"
                  />
                </div>
              </div>
            </div>
          </section>
        </Reveal>

        {/* ─── FEATURE: Connecter ─── */}
        <Reveal>
          <section className="py-6 sm:py-12 lg:py-16 px-4 sm:px-6 relative z-10">
            <div className="container mx-auto max-w-5xl">
              <div className="brozr-feature-row flex items-center justify-center gap-6 sm:gap-10 p-4 sm:p-6 rounded-2xl hover:bg-[rgba(180,185,200,0.02)] transition-colors duration-500">
                <div className="w-[95px] sm:w-[125px] lg:w-[155px] flex-shrink-0 transform -rotate-2">
                  <img
                    src="/UIvf2.png"
                    alt="Communauté"
                    className="w-full h-auto rounded-2xl shadow-2xl ring-1 ring-[rgba(180,185,200,0.08)]"
                  />
                </div>
                <div className="text-left max-w-[260px] sm:max-w-[340px]">
                  <p className="text-lg sm:text-2xl lg:text-3xl font-black leading-snug tracking-tight" style={{ ...fontSatoshi, color: '#E8E8EE' }}>
                    Suis, match, crée ton <ChromeAccent>crew</ChromeAccent>
                  </p>
                  <p className="text-xs sm:text-sm lg:text-base leading-relaxed mt-2" style={{ ...fontOutfit, color: 'rgba(200,205,220,0.6)' }}>
                    Reste en contact et amuse-toi avec ceux qui t'ont fait kiffer. Follow tes bros, messages privés, invitations de live cam
                  </p>
                </div>
              </div>
            </div>
          </section>
        </Reveal>

        {/* ─── FEATURE: Monétiser ─── */}
        <Reveal>
          <section className="py-8 sm:py-14 lg:py-18 px-4 sm:px-6 relative z-10">
            <div className="container mx-auto max-w-5xl">
              <div className="brozr-feature-row flex items-center justify-center gap-6 sm:gap-10 p-4 sm:p-6 rounded-2xl hover:bg-[rgba(180,185,200,0.02)] transition-colors duration-500">
                <div className="text-left max-w-[260px] sm:max-w-[340px]">
                  <p className="text-lg sm:text-2xl lg:text-3xl font-black leading-snug tracking-tight" style={{ ...fontSatoshi, color: '#E8E8EE' }}>
                    Tips, shows privés : ton contenu, tes <ChromeAccent>revenus</ChromeAccent>
                  </p>
                  <p className="text-xs sm:text-sm lg:text-base leading-relaxed mt-2" style={{ ...fontOutfit, color: 'rgba(200,205,220,0.6)' }}>
                    Deviens créateur et gagne de l'argent avec tes lives. Tips en direct, shows en streaming
                  </p>
                </div>
                <div className="w-[95px] sm:w-[125px] lg:w-[155px] flex-shrink-0 transform rotate-2">
                  <video
                    ref={videoRef2}
                    src="/Brozr_Preview_UI3.mp4"
                    autoPlay loop muted playsInline
                    webkit-playsinline="true"
                    preload="auto"
                    className="w-full h-auto rounded-2xl shadow-2xl ring-1 ring-[rgba(180,185,200,0.08)]"
                  />
                </div>
              </div>
            </div>
          </section>
        </Reveal>

        {/* ─── FINAL CTA ─── */}
        <Reveal>
          <section className="py-12 sm:py-16 lg:py-20 px-4 sm:px-6 relative overflow-hidden z-10">
            <div
              className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[450px] h-[450px] rounded-full blur-[60px]"
              style={{ background: 'radial-gradient(circle, rgba(180,185,200,0.04), transparent)' }}
            />
            <div className="container mx-auto max-w-3xl text-center space-y-4 sm:space-y-6 relative z-10">
              <div className="inline-flex items-center gap-2 px-4 py-2 rounded-full bg-[rgba(52,211,153,0.06)] border border-[rgba(52,211,153,0.15)] backdrop-blur-md">
                <span className="relative flex h-2 w-2">
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" />
                  <span className="relative inline-flex rounded-full h-2 w-2 bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.25)]" />
                </span>
                <span className="text-xs font-medium text-[rgba(52,211,153,0.7)]" style={fontOutfit}>
                  <span className="text-emerald-400 font-bold tabular-nums">{onlineCount.toLocaleString()}</span> bros en ligne
                </span>
              </div>

              <h3
                className="text-3xl sm:text-5xl lg:text-6xl font-black tracking-tighter"
                style={{ ...fontSatoshi, color: '#E8E8EE' }}
              >
                Prêt à <ChromeAccent>vibrer</ChromeAccent> ?
              </h3>
              <p className="text-sm sm:text-base leading-relaxed" style={{ ...fontOutfit, color: 'rgba(200,205,220,0.6)' }}>
                Rejoins des milliers de bros déjà connectés.
              </p>

              <div className="pt-3 sm:pt-4 space-y-3">
                <Button
                  onClick={handleGoogleAuth}
                  data-testid="final-cta-btn"
                  className="brozr-chrome-btn brozr-glow-btn h-12 sm:h-14 px-8 sm:px-12 rounded-2xl text-sm sm:text-base font-bold hover:scale-[1.03] transition-all duration-300 w-full sm:w-auto shadow-[0_0_40px_rgba(180,185,200,0.08)]"
                  style={fontOutfit}
                >
                  Commencer gratuitement
                </Button>
                <p className="text-[10px] sm:text-xs leading-relaxed" style={{ color: '#4A4A5E' }}>
                  Gratuit · Commence en 30 secondes
                </p>
              </div>
            </div>
          </section>
        </Reveal>

        {/* ─── FOOTER ─── */}
        <footer className="border-t border-[rgba(180,185,200,0.08)] py-8 sm:py-10 px-4 sm:px-6 relative z-10">
          <div className="container mx-auto max-w-5xl">
            <div className="flex flex-col sm:flex-row justify-between items-center gap-4 sm:gap-6">
              <div className="flex items-center gap-2.5">
                <span className="text-lg font-bold" style={{ ...fontPoppins, color: '#6A6A7E' }}>Brozr</span>
                <span className="text-xs" style={{ color: '#4A4A5E' }}>© 2026 · Real Guys. Real Fun.</span>
              </div>
              <div className="flex gap-6 text-xs" style={{ color: '#4A4A5E' }}>
                <a href="#" className="hover:text-[#9A9AAE] transition-colors duration-300">CGV</a>
                <a href="#" className="hover:text-[#9A9AAE] transition-colors duration-300">Confidentialité</a>
                <a href="#" className="hover:text-[#9A9AAE] transition-colors duration-300">Contact</a>
              </div>
            </div>
          </div>
        </footer>
      </div>
    </>
  );
};

export default Hero;
