import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';

const resources = {
  en: {
    translation: {
      hero: {
        tagline: 'Real Guy. Real Fun.',
        subtitle: 'Connect. Filter. Match.',
        description: 'Premium live cam platform for authentic connections.',
        cta_signup: 'Sign Up',
        cta_login: 'Log In'
      },
      auth: {
        signup: 'Create Account',
        login: 'Login',
        email: 'Email',
        password: 'Password',
        confirm_password: 'Confirm Password',
        or_continue_with: 'Or continue with',
        already_have_account: 'Already have an account?',
        dont_have_account: "Don't have an account?",
        forgot_password: 'Forgot password?'
      },
      onboarding: {
        age_verification: 'Age Verification',
        enter_birthdate: 'Enter your birth date',
        must_be_18: 'You must be 18+ to use Brozr',
        continue: 'Continue',
        profile_setup: 'Profile Setup',
        display_name: 'Display Name',
        bio: 'Bio (optional)',
        select_kinks: 'Select your kinks (max 5)',
        add_photo: 'Add Photo',
        skip_for_now: 'Skip for now'
      },
      live: {
        go_live: 'Go Live',
        next: 'Next',
        like: 'Like',
        follow: 'Follow',
        send_juice: 'Send Juice',
        report: 'Report',
        filters: 'Filters',
        searching: 'Searching...',
        no_users: 'No users available right now'
      },
      premium: {
        title: 'Go Premium',
        unlock_features: 'Unlock Advanced Filters',
        unlimited_calls: 'Unlimited 1:1 Matches',
        age_filter: 'Age Filter',
        location_filter: 'Location Filter',
        kinks_filter: 'Kinks Filter',
        subscribe: 'Subscribe',
        per_month: '/month'
      },
      profile: {
        my_profile: 'My Profile',
        edit: 'Edit',
        settings: 'Settings',
        logout: 'Logout',
        followers: 'Followers',
        following: 'Following',
        matches: 'Matches'
      },
      nav: {
        live: 'Live',
        space: 'Space',
        profile: 'Profile'
      }
    }
  },
  fr: {
    translation: {
      hero: {
        tagline: 'Vrai Mec. Vrai Plaisir.',
        subtitle: 'Vibrer. Monétiser. Fédérer.',
        description: 'Plateforme live cam next-gen, entre bros.',
        cta_signup: "S'inscrire",
        cta_login: 'Se connecter'
      },
      auth: {
        signup: 'Créer un compte',
        login: 'Connexion',
        email: 'Email',
        password: 'Mot de passe',
        confirm_password: 'Confirmer le mot de passe',
        or_continue_with: 'Ou continuer avec',
        already_have_account: 'Vous avez déjà un compte ?',
        dont_have_account: "Vous n'avez pas de compte ?",
        forgot_password: 'Mot de passe oublié ?'
      },
      onboarding: {
        age_verification: 'Vérification de l\'âge',
        enter_birthdate: 'Entrez votre date de naissance',
        must_be_18: 'Vous devez avoir 18 ans ou plus pour utiliser Brozr',
        continue: 'Continuer',
        profile_setup: 'Configuration du profil',
        display_name: 'Nom d\'affichage',
        bio: 'Bio (facultatif)',
        select_kinks: 'Sélectionnez vos kinks (max 5)',
        add_photo: 'Ajouter une photo',
        skip_for_now: 'Passer pour l\'instant'
      },
      live: {
        go_live: 'Go Live',
        next: 'Suivant',
        like: 'Liker',
        follow: 'Suivre',
        send_juice: 'Envoyer du Juice',
        report: 'Signaler',
        filters: 'Filtres',
        searching: 'Recherche en cours...',
        no_users: 'Aucun utilisateur disponible pour le moment'
      },
      premium: {
        title: 'Passer Premium',
        unlock_features: 'Débloquer les fonctionnalités Premium',
        unlimited_calls: 'Appels 1:1 illimités',
        age_filter: 'Filtre d\'âge',
        location_filter: 'Filtre de localisation',
        kinks_filter: 'Filtre de kinks',
        subscribe: 'S\'abonner',
        per_month: '/mois'
      },
      juice: {
        title: 'Boutique Juice',
        buy_juice: 'Acheter du Juice',
        balance: 'Solde',
        send: 'Envoyer',
        earn: 'Gagner'
      },
      profile: {
        my_profile: 'Mon profil',
        edit: 'Modifier',
        settings: 'Paramètres',
        logout: 'Déconnexion',
        juice_earnings: 'Gains en Juice',
        followers: 'Abonnés',
        following: 'Abonnements'
      },
      nav: {
        live: 'Live',
        space: 'Space',
        profile: 'Profil'
      }
    }
  },
  de: {
    translation: {
      hero: {
        tagline: 'Echter Typ. Echter Spaß.',
        subtitle: 'Vibrieren. Monetarisieren. Vereinen.',
        description: 'Next-Gen-Live-Cam-Plattform unter Bros.',
        cta_signup: 'Registrieren',
        cta_login: 'Anmelden'
      },
      auth: {
        signup: 'Konto erstellen',
        login: 'Anmeldung',
        email: 'E-Mail',
        password: 'Passwort',
        confirm_password: 'Passwort bestätigen',
        or_continue_with: 'Oder fortfahren mit',
        already_have_account: 'Bereits ein Konto?',
        dont_have_account: 'Noch kein Konto?',
        forgot_password: 'Passwort vergessen?'
      },
      onboarding: {
        age_verification: 'Altersüberprüfung',
        enter_birthdate: 'Geben Sie Ihr Geburtsdatum ein',
        must_be_18: 'Sie müssen 18+ sein, um Brozr zu nutzen',
        continue: 'Weiter',
        profile_setup: 'Profil-Einrichtung',
        display_name: 'Anzeigename',
        bio: 'Bio (optional)',
        select_kinks: 'Wählen Sie Ihre Kinks (max 5)',
        add_photo: 'Foto hinzufügen',
        skip_for_now: 'Vorerst überspringen'
      },
      live: {
        go_live: 'Go Live',
        next: 'Weiter',
        like: 'Mögen',
        follow: 'Folgen',
        send_juice: 'Juice senden',
        report: 'Melden',
        filters: 'Filter',
        searching: 'Suche läuft...',
        no_users: 'Derzeit keine Benutzer verfügbar'
      },
      premium: {
        title: 'Premium werden',
        unlock_features: 'Premium-Funktionen freischalten',
        unlimited_calls: 'Unbegrenzte 1:1-Anrufe',
        age_filter: 'Altersfilter',
        location_filter: 'Standortfilter',
        kinks_filter: 'Kinks-Filter',
        subscribe: 'Abonnieren',
        per_month: '/Monat'
      },
      juice: {
        title: 'Juice Shop',
        buy_juice: 'Juice kaufen',
        balance: 'Guthaben',
        send: 'Senden',
        earn: 'Verdienen'
      },
      profile: {
        my_profile: 'Mein Profil',
        edit: 'Bearbeiten',
        settings: 'Einstellungen',
        logout: 'Abmelden',
        juice_earnings: 'Juice-Einnahmen',
        followers: 'Follower',
        following: 'Folge ich'
      },
      nav: {
        live: 'Live',
        space: 'Space',
        profile: 'Profil'
      }
    }
  },
  it: {
    translation: {
      hero: {
        tagline: 'Vero Ragazzo. Vero Divertimento.',
        subtitle: 'Vibrare. Monetizzare. Unire.',
        description: 'Piattaforma live cam di nuova generazione, tra bros.',
        cta_signup: 'Iscriviti',
        cta_login: 'Accedi'
      },
      auth: {
        signup: 'Crea account',
        login: 'Accesso',
        email: 'Email',
        password: 'Password',
        confirm_password: 'Conferma password',
        or_continue_with: 'Oppure continua con',
        already_have_account: 'Hai già un account?',
        dont_have_account: 'Non hai un account?',
        forgot_password: 'Password dimenticata?'
      },
      onboarding: {
        age_verification: 'Verifica dell\'età',
        enter_birthdate: 'Inserisci la tua data di nascita',
        must_be_18: 'Devi avere 18+ anni per usare Brozr',
        continue: 'Continua',
        profile_setup: 'Configurazione profilo',
        display_name: 'Nome visualizzato',
        bio: 'Bio (opzionale)',
        select_kinks: 'Seleziona i tuoi kinks (max 5)',
        add_photo: 'Aggiungi foto',
        skip_for_now: 'Salta per ora'
      },
      live: {
        go_live: 'Go Live',
        next: 'Avanti',
        like: 'Mi piace',
        follow: 'Segui',
        send_juice: 'Invia Juice',
        report: 'Segnala',
        filters: 'Filtri',
        searching: 'Ricerca in corso...',
        no_users: 'Nessun utente disponibile al momento'
      },
      premium: {
        title: 'Diventa Premium',
        unlock_features: 'Sblocca funzionalità Premium',
        unlimited_calls: 'Chiamate 1:1 illimitate',
        age_filter: 'Filtro età',
        location_filter: 'Filtro posizione',
        kinks_filter: 'Filtro kinks',
        subscribe: 'Iscriviti',
        per_month: '/mese'
      },
      juice: {
        title: 'Negozio Juice',
        buy_juice: 'Compra Juice',
        balance: 'Saldo',
        send: 'Invia',
        earn: 'Guadagna'
      },
      profile: {
        my_profile: 'Il mio profilo',
        edit: 'Modifica',
        settings: 'Impostazioni',
        logout: 'Esci',
        juice_earnings: 'Guadagni Juice',
        followers: 'Follower',
        following: 'Seguendo'
      },
      nav: {
        live: 'Live',
        space: 'Space',
        profile: 'Profilo'
      }
    }
  },
  es: {
    translation: {
      hero: {
        tagline: 'Chico Real. Diversión Real.',
        subtitle: 'Vibrar. Monetizar. Unir.',
        description: 'Plataforma de live cam de nueva generación, entre bros.',
        cta_signup: 'Registrarse',
        cta_login: 'Iniciar sesión'
      },
      auth: {
        signup: 'Crear cuenta',
        login: 'Iniciar sesión',
        email: 'Correo electrónico',
        password: 'Contraseña',
        confirm_password: 'Confirmar contraseña',
        or_continue_with: 'O continuar con',
        already_have_account: '¿Ya tienes una cuenta?',
        dont_have_account: '¿No tienes una cuenta?',
        forgot_password: '¿Olvidaste tu contraseña?'
      },
      onboarding: {
        age_verification: 'Verificación de edad',
        enter_birthdate: 'Ingresa tu fecha de nacimiento',
        must_be_18: 'Debes tener 18+ años para usar Brozr',
        continue: 'Continuar',
        profile_setup: 'Configuración de perfil',
        display_name: 'Nombre de usuario',
        bio: 'Biografía (opcional)',
        select_kinks: 'Selecciona tus kinks (máx 5)',
        add_photo: 'Agregar foto',
        skip_for_now: 'Omitir por ahora'
      },
      live: {
        go_live: 'Go Live',
        next: 'Siguiente',
        like: 'Me gusta',
        follow: 'Seguir',
        send_juice: 'Enviar Juice',
        report: 'Reportar',
        filters: 'Filtros',
        searching: 'Buscando...',
        no_users: 'No hay usuarios disponibles en este momento'
      },
      premium: {
        title: 'Hazte Premium',
        unlock_features: 'Desbloquear funciones Premium',
        unlimited_calls: 'Llamadas 1:1 ilimitadas',
        age_filter: 'Filtro de edad',
        location_filter: 'Filtro de ubicación',
        kinks_filter: 'Filtro de kinks',
        subscribe: 'Suscribirse',
        per_month: '/mes'
      },
      juice: {
        title: 'Tienda Juice',
        buy_juice: 'Comprar Juice',
        balance: 'Saldo',
        send: 'Enviar',
        earn: 'Ganar'
      },
      profile: {
        my_profile: 'Mi perfil',
        edit: 'Editar',
        settings: 'Configuración',
        logout: 'Cerrar sesión',
        juice_earnings: 'Ganancias de Juice',
        followers: 'Seguidores',
        following: 'Siguiendo'
      },
      nav: {
        live: 'Live',
        space: 'Space',
        profile: 'Perfil'
      }
    }
  }
};

i18n
  .use(initReactI18next)
  .init({
    resources,
    lng: 'en',
    fallbackLng: 'en',
    interpolation: {
      escapeValue: false
    }
  });

export default i18n;
