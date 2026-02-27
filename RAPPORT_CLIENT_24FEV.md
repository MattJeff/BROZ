# Rapport de preparation - Call du 24 fevrier 2026

**Pour : Niels FRYDA**
**De : Mathis HIGUINEN**
**Date : 24 fevrier 2026**

---

## I. FONCTIONNALITES

### 1. Review des travaux a date

#### Etat des fonctionnalites core MVP

| Fonctionnalite | Statut | Commentaire |
|---|---|---|
| **Matching 1:1 aléatoire** | OK | Queue temps reel via Socket.IO, algorithme de scoring |
| **Matching avec filtres** | OK | Pays (gratuit), age, distance, kinks (premium) |
| **Video call P2P (WebRTC)** | OK | Peer-to-peer direct, signaling via Socket.IO |
| **Chat pendant le call** | OK | Messages texte en temps reel pendant le 1:1 |
| **Like pendant le call** | OK | Envoyer un like au partenaire, compteur total_likes |
| **Follow pendant le call** | OK | Envoyer/accepter demande de follow en call |
| **Invitation Live Cam depuis Space** | OK | Bouton "Live Cam" sur le profil d'un Bro, systeme call-invite/accept/decline |
| **Inscription / Connexion** | OK | Email + mot de passe, verification email par code, Google OAuth, forgot/reset password |
| **Onboarding** | OK | Pseudo, date de naissance, bio, kinks, pays, photo de profil |
| **Profil utilisateur** | OK | Photo, pseudo, bio, kinks, age, pays, likes recus |
| **Systeme de Follow (Mes Bros)** | OK | Demande, acceptation, suppression, auto-accept mutuel |
| **Messagerie** | OK | DM 1:1, messages groupe, envoi media (images/videos), medias prives (floute/reveal), pagination, read receipts |
| **Notifications in-app** | OK | Cloche avec badge, 7 types (follow, like, message, livecam, welcome...), mark as read |
| **Signalement (Report)** | OK | 3 points de signalement : profil (Space), message (Space), live cam 1:1 (VideoCall). 5 categories de motif + commentaire optionnel |

#### Fonctionnalites backend supplementaires implementees

| Fonctionnalite | Statut | Commentaire |
|---|---|---|
| **API Gateway** | OK | Reverse proxy, rate limiting (free/premium), CORS, JWT enforcement |
| **Moderation backend (API admin)** | OK | Liste/review/action des signalements, sanctions (warning, ban 1h/24h/30d/permanent), audit log, stats dashboard |
| **Analytics** | OK | Tracking events (inscription, match, like, message, report...), stats DAU/WAU/MAU, agregation journaliere |
| **Presence en ligne** | OK | Statut online/offline via Redis, visible dans la liste des Bros |

#### Ce qui reste a faire / points d'attention

| Element | Statut | Detail |
|---|---|---|
| **Dashboard moderation frontend** | OK | Interface web admin complete a `/admin` : stats (signalements en attente, sanctions actives, signalements du jour), liste des signalements avec filtrage par statut, dialog de review avec sanctions (avertissement, ban 1h/24h/30d/permanent), onglet sanctions actives avec possibilite de lever, journal d'audit. Role admin lu depuis la base de donnees. |
| **Moderation passive (voir les live cams)** | NON FAIT | Necessite une integration specifique WebRTC (le moderateur se connecte comme "spectateur invisible" au flux P2P). Faisable mais complexe. **Estimation : 500 EUR comme evoque.** |
| **Push notifications (mobile)** | NON FAIT | Notifications in-app uniquement. Pas de push FCM/APNs. Hors scope MVP initial. |
| **Distance/km reelle** | SIMPLIFIE | Le matching utilise un facteur distance simplifie (1.0) pour le MVP. La geolocalisation precise necessite l'API de localisation cote client. |
| **Recherche utilisateur par pseudo** | PARTIEL | La recherche existe dans la liste "Mes Bros" (filtrage local), mais il n'y a pas de **recherche globale** sur tous les utilisateurs de la plateforme. |
| **PPV (Pay-Per-View)** | PARTIEL | Le mecanisme de media prive/floute/reveal existe dans la messagerie. Le reveal est **gratuit** pour l'instant car le systeme de Juices n'est pas encore en place. La brique technique est prete, il suffira de brancher le systeme de paiement dessus. |

#### Bug fixe aujourd'hui

**Race condition socketioxide** : Bug critique identifie et corrige dans le service de matching. Les handlers d'evenements Socket.IO etaient enregistres apres des operations Redis asynchrones, ce qui causait une perte intermittente des evenements `join-queue`. Le fix deplace l'enregistrement des handlers en premier, avant toute operation async. Le matching fonctionne maintenant de maniere fiable.

---

### 2. Debug / QA & Modifications UI/UX

#### Modus operandi pour les modifications mineures

Proposition de process :
1. **Niels** liste les modifications sur un document partage (Notion, Google Doc, ou GitHub Issues)
2. **Mathis** priorise et implemente par batch
3. Test en local → push sur branche de dev → validation
4. Merge sur main → deploiement sur VPS

#### Process QA / Recette

Proposition de workflow :
- **Environnement de dev** : `localhost:3000` (frontend) + Docker backend local
- **Environnement de staging/prod** : VPS avec le meme Docker Compose (ou K3s si besoin de scalabilite plus tard)
- **Recette** : Acces a l'URL du VPS pour tester en conditions reelles avant le "go live" public
- Le deploiement se fait via `docker compose up -d --build` sur le VPS (CI/CD optionnel pour la suite)

#### Modifications mineures demandees

**a. Ordre des Bros dans Space**
- **Etat actuel** : Les Bros s'affichent sans tri specifique (ordre d'insertion en base, donc les plus anciens en premier)
- **Correction** : Trier par date d'ajout decroissante (les plus recents en premier). Modification simple cote backend (ajouter un `ORDER BY created_at DESC` sur la requete) + eventuellement cote frontend.
- **Estimation** : < 1 heure

**b. Bouton "Ajouter un Bro" + Recherche par pseudo**
- **Etat actuel** : La recherche existe en local (filtrage dans "Mes Bros"), mais pas de recherche globale
- **A developper** :
  - Bouton "+" a cote de "Voir tout >" dans la section "Mes Bros"
  - Ecran/modal de recherche avec champ texte
  - Endpoint backend `GET /api/users/search?q=pseudo` pour rechercher parmi tous les utilisateurs
  - Affichage des resultats avec bouton "Ajouter" (envoie une demande de follow)
- **Estimation** : 3-4 heures (endpoint backend + modal frontend + integration)
- **Complexite** : Modere - faisable dans le scope MVP

---

### 3. Moderation

#### Dashboard de moderation

- **Backend** : COMPLET. L'API admin offre :
  - Liste des signalements (filtrage par statut pending/actioned/dismissed)
  - Detail d'un signalement
  - Action sur un signalement (sanction ou rejet)
  - 5 types de sanctions : warning, ban 1h, ban 24h, ban 30d, ban permanent
  - Les sanctions sont appliquees automatiquement au login (l'utilisateur banni ne peut plus se connecter)
  - Audit log de toutes les actions admin
  - Stats : nombre de signalements en attente, sanctions actives, signalements du jour

- **Frontend** : FAIT. Page `/admin` dans le frontend React avec interface complete de moderation (stats, signalements, sanctions, audit log). Accessible uniquement aux comptes avec role `admin`.

#### Points de signalement (3/3 implementes)

| Point de signalement | Statut | Detail |
|---|---|---|
| **Profil** (depuis Space) | OK | Menu "..." sur le profil d'un Bro → Signaler |
| **Message** (depuis Space) | OK | Option de signalement dans la conversation |
| **Live Cam 1:1** (depuis VideoCall) | OK | Bouton drapeau pendant le call, 5 categories + commentaire |

#### Cote moderation lors d'un signalement Live Cam

Quand un utilisateur signale pendant un live cam 1:1 :
- Le signalement est enregistre en base avec le `match_session_id` (identifiant unique de la session de matching)
- Le moderateur voit dans l'API : qui a signale, qui est signale, le motif, le commentaire, et l'ID de session
- **Pas de capture d'ecran/video automatique** pour le moment (le flux est P2P, il n'y a pas de serveur intermediaire qui enregistre)

#### Moderation passive (voir les live cams sans etre vu)

- **Etat actuel** : Non implemente
- **Faisabilite** : Techniquement faisable via le SFU (serveur media intermediaire). Le moderateur se connecterait comme "subscriber" invisible sur un flux existant. Necessite :
  - L'integration du SFU pour le 1:1 (actuellement P2P direct)
  - OU un mecanisme de "forwarding" du flux vers le moderateur
- **Impact** : Passer le 1:1 en P2P → SFU augmente la latence et la charge serveur. Alternative : ne l'activer que sur demande (quand un moderateur veut observer un flux specifique)
- **Estimation** : Feature complexe, ~500 EUR semble raisonnable pour une version basique

---

## II. ARCHITECTURE

### 1. Serveurs VPS

#### Architecture actuelle

9 microservices Rust (Axum) + PostgreSQL + Redis + RabbitMQ + MinIO (stockage objets) + SFU WebRTC.

Le tout tourne dans Docker Compose. Chaque service est un binaire Rust compile, tres leger en ressources (quelques Mo de RAM par service).

#### Clarification SFU vs P2P

- **Matching 1:1** : P2P direct (WebRTC peer-to-peer). **Pas besoin de SFU.** Le signaling passe par Socket.IO, mais le flux video/audio va directement d'un navigateur a l'autre.
- **Space calls 1:1** : Egalement P2P. Meme logique.
- **Play Show (1:many)** : Necessite un SFU car un seul streameur doit diffuser vers N viewers. Le SFU recoit le flux du streameur et le redistribue.

**Conclusion** : Pour le MVP (1:1 uniquement), **pas besoin de serveur SFU**. On economise ~129 EUR/mois.

#### Clarification sur les calls Space en version finale

Les calls depuis Space resteront toujours en 1:1 P2P. La seule passerelle necessaire entre Play Show (1:many) et un appel 1:1 serait :
1. Un viewer du Play Show clique "Appeler" sur le streameur
2. Le systeme cree un appel P2P classique entre les deux (identique au matching)
3. Pas besoin de SFU pour cette transition — c'est juste un nouveau call P2P initie depuis un contexte different

#### Recommandation serveurs (vision MVP, maitrise des couts)

Hebergeurs recommandes : **AbeloHost** (Pays-Bas, specialise adulte) + **BuyVM** (Luxembourg, budget).

**Option MVP Budget — ~83 EUR/mois :**

| Role | Hebergeur | Specs | Prix/mois |
|---|---|---|---|
| **Tout-en-un** (9 services + BDD + Redis + RabbitMQ) | BuyVM Luxembourg | 4 cores, 16GB RAM, 320GB SSD, bande passante illimitee 10Gbps | ~55 EUR |
| **Stockage medias** (MinIO/S3) | Wasabi | 1 TB | ~6 EUR |
| **CDN** (assets statiques) | Bunny.net | Pay-as-you-go | ~5-10 EUR |
| **DNS + SSL** | Cloudflare | Free tier | 0 EUR |
| **Domaine** | - | brozr.com | ~12 EUR/an |
| **Total** | | | **~83 EUR/mois** |

> Les services Rust sont extremement legers. 9 microservices + PostgreSQL + Redis + RabbitMQ tiennent largement sur 16GB RAM pour le MVP. On peut monter en puissance (separer la BDD sur son propre serveur, ajouter des noeuds) quand le trafic le justifie.

**Option MVP Confortable — ~160 EUR/mois :**

| Role | Hebergeur | Specs | Prix/mois |
|---|---|---|---|
| **Backend** (9 services) | AbeloHost VPS | 8 cores, 16GB RAM, 240GB SSD | 80 EUR |
| **Base de donnees** (PostgreSQL + Redis + RabbitMQ) | BuyVM Luxembourg | 4 cores, 16GB RAM, 320GB SSD | ~55 EUR |
| **Stockage + CDN** | Wasabi + Bunny.net | | ~15 EUR |
| **Total** | | | **~160 EUR/mois** |

#### Capacite estimee (3-5k paires simultanées)

- 5 000 paires = 10 000 utilisateurs connectes en simultane
- Le matching passe par Socket.IO (leger, quelques Ko par connexion)
- La video est P2P (pas de bande passante serveur pour le flux video)
- Le serveur gere uniquement : signaling WebRTC, queue matching, messages, presence
- **Avec 16GB RAM et 4 cores, on peut facilement gerer 10k connexions Socket.IO simultanees**
- Le goulot d'etranglement sera PostgreSQL (connexions DB), pas les services Rust
- Si besoin, on ajoute du connection pooling (PgBouncer) — gratuit

#### Niveau de confiance

**Eleve.** Les prix sont bases sur les tarifs publics des hebergeurs. BuyVM est tres abordable car pas de marge "adulte" (ils hebergent tout ce qui est legal au Luxembourg). AbeloHost est plus cher mais specialise et offre un support dedie.

Le surdimensionnement est minimal : on prend le strict necessaire pour le MVP, et on scale horizontalement quand le trafic le demande. La stack Docker Compose se transpose directement sur un VPS.

---

## III. OVERVIEW PROJET

### 1. Deadline MVP — 6 mars

#### Etat d'avancement

- **Core features** : 90% fait. Matching, calls, messaging, notifications, profils, follows, likes, signalements — tout fonctionne.
- **Bugs critiques** : Le bug de matching (race condition) vient d'etre corrige. Les tests montrent que le matching fonctionne de maniere fiable.
- **Reste a faire pour le 6 mars** :

| Tache | Estimation | Priorite |
|---|---|---|
| Deploiement sur VPS (Docker Compose + config prod) | 1-2 jours | CRITIQUE |
| ~~Tri des Bros (plus recents en premier)~~ | ~~< 1 heure~~ | ~~FAIT~~ |
| Bouton "Ajouter un Bro" + recherche par pseudo | 3-4 heures | Moyen |
| ~~Dashboard moderation (version basique)~~ | ~~2-3 jours~~ | ~~FAIT~~ |
| QA / recette / fix bugs mineurs | 2-3 jours | CRITIQUE |
| ~~Nettoyage des logs de debug~~ | ~~1 heure~~ | ~~FAIT~~ |

#### Niveau de confiance pour le 6 mars

**Moyen-Haut.** Les fonctionnalites core sont la. Les risques :
- Le deploiement VPS peut reveler des problemes de configuration (SSL, DNS, ports, Docker en prod)
- La QA/recette peut faire emerger des bugs non detectes en local
- Le dashboard moderation est le plus gros morceau restant

**Proposition** : Prioriser dans cet ordre :
1. Deploiement VPS (jours 1-2)
2. QA/recette (jours 2-4)
3. Modifications mineures UI (jour 4)
4. Dashboard moderation (jours 5-8)

Si le dashboard moderation n'est pas pret pour le 6, on peut utiliser Postman/curl en attendant (l'API est complete).

### 2. Suite des travaux

#### Concernant la proposition de paiement

La proposition de Niels :
- 50% a la signature, 50% a la livraison de la montee en charge
- Reequilibrage : 12k EUR pour MVP 2-3 (au lieu de 17.1k), 11k EUR pour MVP 4 (au lieu de 6.9k)

> A discuter lors du call. Points a clarifier :
> - Definition precise des jalons de "montee en charge" (nombre d'utilisateurs ? features ?)
> - Le MVP 4 a 11k EUR inclut-il des features supplementaires par rapport au devis initial ?
> - Garantie de stabilite du MVP 0 pendant le developpement des versions suivantes

#### Stabilite MVP 0 pendant les travaux suivants

Oui, c'est assure par l'architecture microservices :
- Chaque service est independant et deploye separement
- Les nouvelles features (Play Show, Juices, etc.) seront de nouveaux services ou des extensions des services existants
- Pas besoin de toucher au code du matching ou de la messagerie pour ajouter le Play Show
- Deployement blue/green possible si besoin (zero downtime)

---

## POINTS A EVOQUER LORS DU CALL

1. **Demo live** : Proposer une demo en partage d'ecran pendant le call (matching, messaging, profil, signalement)
2. **Choix du VPS** : Valider ensemble l'option budget (~83 EUR) vs confortable (~160 EUR)
3. **Dashboard moderation** : Valider la priorite et l'approche (Retool rapide vs interface custom)
4. **Moderation passive** : Confirmer le budget de 500 EUR et la priorite
5. **Feature "Ajouter un Bro"** : Valider le scope (SVG du bouton a fournir par Niels)
6. **Planning 6 mars** : Aligner sur les priorites restantes
