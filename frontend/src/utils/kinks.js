// Kinks organisés par catégories avec le bon ordre
// ORDRE: Actif, Passif, Versatile → Dominateur, Soumis

export const KINK_CATEGORIES = {
  roles: {
    label: "Rôles & dynamiques",
    emoji: "🔥",
    kinks: [
      // Ordre spécifique demandé
      { id: 'actif', label: 'Actif', matchWith: ['passif', 'versatile'] },
      { id: 'passif', label: 'Passif', matchWith: ['actif', 'versatile'] },
      { id: 'versatile', label: 'Versatile', matchWith: ['actif', 'passif', 'versatile'] },
      { id: 'dominateur', label: 'Dominateur', matchWith: ['soumis'] },
      { id: 'soumis', label: 'Soumis', matchWith: ['dominateur'] },
    ]
  },
  orientation: {
    label: "Orientation",
    emoji: "🌈",
    kinks: [
      { id: 'hetero', label: 'Hétéro', matchWith: null },
      { id: 'heteroflexible', label: 'Hétéroflexible', matchWith: null },
      { id: 'bi', label: 'Bi', matchWith: null },
      { id: 'gay', label: 'Gay', matchWith: null },
    ]
  },
  profils: {
    label: "Profils & attirances",
    emoji: "💪",
    kinks: [
      { id: 'masculin', label: 'Masculin', matchWith: null },
      { id: 'twink', label: 'Twink', matchWith: null },
      { id: 'minet', label: 'Minet', matchWith: null },
      { id: 'bear', label: 'Bear', matchWith: null },
      { id: 'femboy', label: 'Femboy', matchWith: null },
      { id: 'trans', label: 'Trans', matchWith: null },
      { id: 'bien_monte', label: 'Bien monté', matchWith: null },
    ]
  },
  visibilite: {
    label: "Visibilité",
    emoji: "👀",
    kinks: [
      { id: 'no_face', label: 'No Face', matchWith: ['no_face', 'discret'] },
      { id: 'discret', label: 'Discret', matchWith: ['no_face', 'discret'] },
      { id: 'avec_visage', label: 'Avec visage', matchWith: ['avec_visage'] },
    ]
  },
  pratiques: {
    label: "Pratiques & kinks",
    emoji: "🎭",
    kinks: [
      { id: 'bdsm', label: 'BDSM', matchWith: ['bdsm'] },
      { id: 'branle_bros', label: 'Branle entre Bros', matchWith: ['branle_bros'] },
      { id: 'jeu_roles', label: 'Jeu de rôles', matchWith: ['jeu_roles'] },
      { id: 'edging', label: 'Edging', matchWith: ['edging'] },
      { id: 'exhib', label: 'Exhib', matchWith: ['exhib'] },
      { id: 'jouet', label: 'Jouet', matchWith: ['jouet'] },
      { id: 'brutal', label: 'Brutal', matchWith: ['brutal'] },
      { id: 'pig', label: 'Pig', matchWith: ['pig'] },
      { id: 'chastete', label: 'Chasteté', matchWith: ['dominateur'] },
      { id: 'dirty_talk', label: 'Dirty talk', matchWith: ['dirty_talk'] },
      { id: 'verbal', label: 'Verbal', matchWith: ['verbal'] },
    ]
  },
  fetishes: {
    label: "Fétiches",
    emoji: "👃",
    kinks: [
      { id: 'odeurs', label: 'Odeurs', matchWith: ['odeurs'] },
      { id: 'aisselles', label: 'Aisselles', matchWith: ['aisselles'] },
      { id: 'pieds', label: 'Pieds', matchWith: ['pieds'] },
      { id: 'uro', label: 'Uro', matchWith: ['uro'] },
    ]
  }
};

// Liste plate pour l'affichage dans les UI (avec le bon ordre)
export const KINKS_FLAT = [
  { cat: "Rôles & dynamiques", emoji: "🔥", items: ["Actif", "Passif", "Versatile", "Dominateur", "Soumis"] },
  { cat: "Orientation", emoji: "🌈", items: ["Hétéro", "Hétéroflexible", "Bi", "Gay"] },
  { cat: "Profils & attirances", emoji: "💪", items: ["Masculin", "Twink", "Minet", "Bear", "Femboy", "Trans", "Bien monté"] },
  { cat: "Visibilité", emoji: "👀", items: ["No Face", "Discret", "Avec visage"] },
  { cat: "Pratiques & kinks", emoji: "🎭", items: ["BDSM", "Branle entre Bros", "Jeu de rôles", "Edging", "Exhib", "Jouet", "Brutal", "Pig", "Chasteté", "Dirty talk", "Verbal"] },
  { cat: "Fétiches", emoji: "👃", items: ["Odeurs", "Aisselles", "Pieds", "Uro"] },
];

// Liste plate de tous les kinks (juste les labels)
export const KINKS = Object.values(KINK_CATEGORIES).flatMap(
  category => category.kinks.map(k => k.label)
);

// Helper pour obtenir tous les kinks avec leurs IDs
export const getAllKinks = () => {
  return Object.values(KINK_CATEGORIES).flatMap(category => category.kinks);
};

// Helper pour obtenir les kinks compatibles
export const getMatchableKinks = (userKinkIds) => {
  const allKinks = getAllKinks();
  const matchable = new Set();
  
  userKinkIds.forEach(kinkId => {
    const kink = allKinks.find(k => k.id === kinkId);
    if (kink && kink.matchWith) {
      kink.matchWith.forEach(matchId => matchable.add(matchId));
    }
  });
  
  return Array.from(matchable);
};

/**
 * Détermine quels kinks du partenaire correspondent à ce que le user recherche (Looking for)
 * @param {string[]} partnerKinks - Les kinks du partenaire (labels)
 * @param {string[]} userLookingFor - Ce que le user recherche dans ses filtres (labels)
 * @returns {{ matching: string[], other: string[] }} - Kinks triés: correspondants d'abord
 */
export const getMatchingKinksForLookingFor = (partnerKinks, userLookingFor) => {
  if (!partnerKinks || partnerKinks.length === 0) {
    return { matching: [], other: [] };
  }
  
  // Si le user n'a pas renseigné de Looking for, aucun kink n'est mis en évidence
  if (!userLookingFor || userLookingFor.length === 0) {
    return { matching: [], other: partnerKinks };
  }
  
  // Normaliser les labels pour la comparaison (lowercase, trim)
  const normalize = (s) => s?.toLowerCase().trim();
  const lookingForNormalized = userLookingFor.map(normalize);
  
  const matching = [];
  const other = [];
  
  partnerKinks.forEach(kink => {
    if (lookingForNormalized.includes(normalize(kink))) {
      matching.push(kink);
    } else {
      other.push(kink);
    }
  });
  
  return { matching, other };
};
