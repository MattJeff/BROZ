/**
 * LiveRelay SDK v0.4.0 — WebRTC SFU streaming client.
 *
 * Features:
 *   - publish()      — send camera/mic to a room
 *   - publish(screen) — share screen to a room
 *   - subscribe()    — receive media from a room
 *   - call()         — 1:1 video call
 *   - conference()   — N-party conference
 *
 * No external dependencies. Works as ES module or script tag.
 */

// ---------------------------------------------------------------------------
// Error class
// ---------------------------------------------------------------------------

/**
 * Structured error for the LiveRelay SDK.
 * Codes: AUTH_ERROR, ROOM_NOT_FOUND, ROOM_FULL, NETWORK_ERROR,
 *        MEDIA_ERROR, TIMEOUT, SERVER_ERROR
 */
class LiveRelayError extends Error {
    /**
     * @param {string} code - One of the SDK error codes
     * @param {string} message - Human-readable description
     * @param {number|null} [httpStatus] - HTTP status code when applicable
     */
    constructor(code, message, httpStatus = null) {
        super(message);
        this.name = 'LiveRelayError';
        this.code = code;
        this.httpStatus = httpStatus;
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Wait for ICE gathering to complete, with a safety timeout.
 * @param {RTCPeerConnection} pc
 * @param {number} [timeoutMs=10000]
 * @returns {Promise<void>}
 */
function waitForIceGathering(pc, timeoutMs = 10000) {
    return new Promise((resolve, reject) => {
        if (pc.iceGatheringState === 'complete') return resolve();

        const timer = setTimeout(() => {
            pc.onicegatheringstatechange = null;
            resolve(); // resolve anyway with what we have
        }, timeoutMs);

        pc.onicegatheringstatechange = () => {
            if (pc.iceGatheringState === 'complete') {
                clearTimeout(timer);
                resolve();
            }
        };
    });
}

/**
 * POST an SDP offer to the server and return the answer.
 * Throws LiveRelayError with structured codes on failure.
 * @param {string} url
 * @param {string} token
 * @param {RTCSessionDescription} offer
 * @param {Object} [extraBody] - Additional fields to merge into the body
 * @returns {Promise<Object>}
 */
async function postSDP(url, token, offer, extraBody = {}) {
    let resp;
    try {
        resp = await fetch(url, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': `Bearer ${token}`
            },
            body: JSON.stringify({
                sdp: offer.sdp,
                type: offer.type,
                ...extraBody
            })
        });
    } catch (e) {
        throw new LiveRelayError('NETWORK_ERROR',
            `Cannot reach server: ${e.message}`);
    }

    if (!resp.ok) {
        let errorData;
        try {
            errorData = await resp.json();
        } catch {
            throw new LiveRelayError('SERVER_ERROR',
                `Server error ${resp.status}`, resp.status);
        }

        const code = errorData?.error?.code || 'SERVER_ERROR';
        const message = errorData?.error?.message || `Server error ${resp.status}`;

        // Map server error codes to SDK error codes
        const codeMap = {
            'auth_header_missing': 'AUTH_ERROR',
            'token_invalid': 'AUTH_ERROR',
            'token_expired': 'AUTH_ERROR',
            'room_not_found': 'ROOM_NOT_FOUND',
            'room_full': 'ROOM_FULL',
            'no_publisher_available': 'ROOM_NOT_FOUND',
        };

        throw new LiveRelayError(
            codeMap[code] || 'SERVER_ERROR',
            message,
            resp.status
        );
    }
    return resp.json();
}

/** Default ICE servers used as fallback when the server is unreachable. */
const DEFAULT_ICE_SERVERS = [{ urls: 'stun:stun.l.google.com:19302' }];

/**
 * Fetch ICE server configuration from the LiveRelay API.
 *
 * The server returns STUN and TURN credentials (including embedded TURN
 * if configured).  Falls back to the default public STUN server on error.
 *
 * @param {string} serverUrl - Base URL of the LiveRelay server
 * @param {string} token - JWT token for authentication
 * @returns {Promise<Array<RTCIceServer>>}
 */
async function fetchIceServers(serverUrl, token) {
    try {
        const resp = await fetch(`${serverUrl}/v1/ice-servers`, {
            headers: { 'Authorization': `Bearer ${token}` }
        });
        if (resp.ok) {
            const data = await resp.json();
            if (data.ice_servers && data.ice_servers.length > 0) {
                return data.ice_servers;
            }
        }
    } catch (_) {
        // Fallback silently -- STUN-only is often sufficient on good networks.
    }
    return DEFAULT_ICE_SERVERS;
}

// ---------------------------------------------------------------------------
// Session classes
// ---------------------------------------------------------------------------

/**
 * Base session -- common state machine shared by all session types.
 */
class BaseSession {
    /** @param {RTCPeerConnection} pc */
    constructor(pc, onStateChange) {
        this._pc = pc;
        this._state = 'connecting';
        this._onStateChange = onStateChange || null;

        this._pc.oniceconnectionstatechange = () => {
            const map = {
                connected: 'connected',
                completed: 'connected',
                disconnected: 'disconnected',
                failed: 'failed',
                closed: 'disconnected'
            };
            const next = map[this._pc.iceConnectionState];
            if (next && next !== this._state) {
                this._state = next;
                if (this._onStateChange) this._onStateChange(this._state);
            }
        };
    }

    get state() {
        return this._state;
    }

    /** Mark session as connected (called after SDP exchange succeeds). */
    _setConnected() {
        if (this._state === 'connecting') {
            this._state = 'connected';
            if (this._onStateChange) this._onStateChange(this._state);
        }
    }

    close() {
        this._pc.close();
        this._state = 'disconnected';
        if (this._onStateChange) this._onStateChange(this._state);
    }
}

/**
 * Mixin: mute / unmute helpers for sessions that send local tracks.
 * Applied to PublishSession, CallSession, and ConferenceSession.
 */
const MutableMixin = (Base) => class extends Base {
    muteAudio() { this._setTrackEnabled('audio', false); }
    unmuteAudio() { this._setTrackEnabled('audio', true); }
    muteVideo() { this._setTrackEnabled('video', false); }
    unmuteVideo() { this._setTrackEnabled('video', true); }

    get isAudioMuted() { return !this._getTrackEnabled('audio'); }
    get isVideoMuted() { return !this._getTrackEnabled('video'); }

    /** @private */
    _setTrackEnabled(kind, enabled) {
        const senders = this._pc.getSenders();
        for (const sender of senders) {
            if (sender.track && sender.track.kind === kind) {
                sender.track.enabled = enabled;
            }
        }
    }

    /** @private */
    _getTrackEnabled(kind) {
        const senders = this._pc.getSenders();
        for (const sender of senders) {
            if (sender.track && sender.track.kind === kind) {
                return sender.track.enabled;
            }
        }
        return false;
    }
};

class PublishSession extends MutableMixin(BaseSession) {
    /**
     * @param {RTCPeerConnection} pc
     * @param {function} onStateChange
     * @param {boolean} isScreen - true if this is a screen share session
     */
    constructor(pc, onStateChange, isScreen = false) {
        super(pc, onStateChange);
        /** True if this session publishes a screen share. */
        this.isScreen = isScreen;
    }
}

class SubscribeSession extends BaseSession {
    constructor(pc, onStateChange) {
        super(pc, onStateChange);
        /** @type {MediaStream|null} Camera video+audio stream. */
        this.cameraStream = null;
        /** @type {MediaStream|null} Screen share stream (if publisher shares screen). */
        this.screenStream = null;
    }
}

class CallSession extends MutableMixin(BaseSession) {}

/**
 * ConferenceSession -- manages N-party conference connections.
 *
 * The main PeerConnection handles publish + subscribe to all peers
 * present at join time. Late joiners are received via additional
 * subscribe-only PeerConnections.
 */
class ConferenceSession extends MutableMixin(BaseSession) {
    /**
     * @param {RTCPeerConnection} pc - Main PeerConnection
     * @param {function} onStateChange
     * @param {Object} conferenceData - { participants, peer_id }
     * @param {LiveRelay} lr - SDK instance for subscribing to newcomers
     * @param {string} token - JWT token
     */
    constructor(pc, onStateChange, conferenceData, lr, token) {
        super(pc, onStateChange);
        /** Your peer_id in this conference. */
        this.peerId = conferenceData.peer_id;
        /** Peer IDs of participants already present when you joined. */
        this.participants = conferenceData.participants || [];
        /** Map of peerId -> MediaStream for each remote participant. */
        this.remoteStreams = new Map();
        /** Map of peerId -> RTCPeerConnection for late-joiner subscriptions. */
        this._subscribePCs = new Map();
        this._lr = lr;
        this._token = token;
        /** Callback: (peerId, stream) when a new participant's stream is available. */
        this.onParticipantJoined = null;
        /** Callback: (peerId) when a participant leaves. */
        this.onParticipantLeft = null;
    }

    /**
     * Subscribe to a new participant who joined after you.
     * @param {string} peerId - The newcomer's peer_id
     * @param {HTMLVideoElement} [element] - Optional element to attach the stream to
     * @returns {Promise<MediaStream>}
     */
    async subscribeToParticipant(peerId, element) {
        const iceServers = await fetchIceServers(this._lr.server, this._token);
        const pc = new RTCPeerConnection({ iceServers });

        const remoteStream = new MediaStream();
        this.remoteStreams.set(peerId, remoteStream);
        this._subscribePCs.set(peerId, pc);

        pc.ontrack = (event) => {
            remoteStream.addTrack(event.track);
            if (element) {
                element.autoplay = true;
                element.playsInline = true;
                element.srcObject = remoteStream;
            }
            if (this.onParticipantJoined) {
                this.onParticipantJoined(peerId, remoteStream);
            }
        };

        // Add recvonly transceivers.
        pc.addTransceiver('video', { direction: 'recvonly' });
        pc.addTransceiver('audio', { direction: 'recvonly' });

        const offer = await pc.createOffer();
        await pc.setLocalDescription(offer);
        await waitForIceGathering(pc);

        const answer = await postSDP(
            `${this._lr.server}/sfu/conference/subscribe`,
            this._token,
            pc.localDescription,
            { target_peer_id: peerId }
        );

        await pc.setRemoteDescription(new RTCSessionDescription(answer));

        this.participants.push(peerId);
        return remoteStream;
    }

    /**
     * Close the conference and all associated PeerConnections.
     */
    close() {
        super.close();
        for (const [, pc] of this._subscribePCs) {
            pc.close();
        }
        this._subscribePCs.clear();
    }
}

// ---------------------------------------------------------------------------
// Main SDK class
// ---------------------------------------------------------------------------

class LiveRelay {
    /**
     * @param {Object|string} options - Server URL string or options object
     * @param {string} options.server - Server URL (e.g., "http://localhost:8080")
     */
    constructor(options) {
        if (typeof options === 'string') {
            options = { server: options };
        }
        if (!options || !options.server) {
            throw new LiveRelayError('AUTH_ERROR', 'server URL is required');
        }
        this.server = options.server.replace(/\/+$/, '');
    }

    /**
     * Publish a media stream to a room.
     *
     * Set `screen: true` to publish a screen share via getDisplayMedia().
     * Screen shares create a separate publisher on the server (with peer_id
     * suffixed "-screen"), so subscribers receive them as a distinct track
     * with stream ID "liverelay-screen".
     *
     * @param {Object} options
     * @param {string} options.token - JWT token (contains room_id and role)
     * @param {MediaStream} [options.stream] - Local MediaStream to publish
     * @param {boolean} [options.video] - Request video via getUserMedia
     * @param {boolean} [options.audio] - Request audio via getUserMedia
     * @param {boolean} [options.screen] - Screen share via getDisplayMedia
     * @param {Object} [options.screenConstraints] - Constraints for getDisplayMedia
     * @param {function} [options.onStateChange] - Callback(state: string)
     * @returns {Promise<PublishSession>}
     */
    async publish(options) {
        let { token, stream, video, audio, screen, screenConstraints, onStateChange } = options;
        if (!token) throw new LiveRelayError('AUTH_ERROR', 'token is required');

        const isScreen = !!screen;

        // Screen share: use getDisplayMedia.
        if (isScreen && !stream) {
            try {
                const constraints = screenConstraints || {
                    video: {
                        cursor: 'always',
                        width: { ideal: 1920 },
                        height: { ideal: 1080 },
                        frameRate: { ideal: 15, max: 30 }
                    },
                    audio: false
                };
                stream = await navigator.mediaDevices.getDisplayMedia(constraints);
            } catch (e) {
                throw new LiveRelayError('MEDIA_ERROR',
                    e.name === 'NotAllowedError'
                        ? 'Screen sharing permission denied.'
                        : `Screen capture failed: ${e.message}`
                );
            }
        }

        // Camera/mic: use getUserMedia.
        if (!isScreen && !stream && (video || audio)) {
            try {
                stream = await navigator.mediaDevices.getUserMedia({
                    video: video || false,
                    audio: audio || false
                });
            } catch (e) {
                throw new LiveRelayError('MEDIA_ERROR',
                    e.name === 'NotAllowedError'
                        ? 'Camera/microphone permission denied. Please allow access in your browser settings.'
                        : `Media capture failed: ${e.message}`
                );
            }
        }
        if (!stream) throw new LiveRelayError('MEDIA_ERROR', 'stream or video/audio/screen options required');

        // Fetch TURN/STUN credentials from the server (with fallback).
        const iceServers = await fetchIceServers(this.server, token);

        const pc = new RTCPeerConnection({ iceServers });
        const session = new PublishSession(pc, onStateChange, isScreen);

        try {
            // Add all tracks from the local stream.
            for (const track of stream.getTracks()) {
                pc.addTrack(track, stream);
            }

            // When screen share ends (user clicks "Stop sharing"), close the session.
            if (isScreen) {
                const videoTrack = stream.getVideoTracks()[0];
                if (videoTrack) {
                    videoTrack.onended = () => {
                        session.close();
                    };
                }
            }

            // Create offer and set local description.
            const offer = await pc.createOffer();
            await pc.setLocalDescription(offer);

            // Wait until ICE candidates are gathered.
            await waitForIceGathering(pc);

            // Exchange SDP with the server.
            // The `screen` flag tells the server to route video to screen_tx.
            const answer = await postSDP(
                `${this.server}/sfu/publish`,
                token,
                pc.localDescription,
                isScreen ? { screen: true } : {}
            );

            await pc.setRemoteDescription(new RTCSessionDescription(answer));
            session._setConnected();
        } catch (err) {
            pc.close();
            session._state = 'failed';
            if (onStateChange) onStateChange('failed');
            throw err;
        }

        return session;
    }

    /**
     * Subscribe to a room and receive media.
     *
     * If the publisher is sharing their screen, the subscriber receives
     * both the camera stream and the screen stream. The SDK distinguishes
     * them by the stream ID:
     *   - "liverelay-cam"    -> camera
     *   - "liverelay-screen" -> screen share
     *
     * @param {Object} options
     * @param {string} options.token - JWT token
     * @param {HTMLVideoElement} options.element - Video element for camera stream
     * @param {HTMLVideoElement} [options.screenElement] - Video element for screen share
     * @param {function} [options.onStateChange] - Callback(state: string)
     * @param {function} [options.onScreenTrack] - Callback(stream: MediaStream) when screen track arrives
     * @returns {Promise<SubscribeSession>}
     */
    async subscribe(options) {
        const { token, element, screenElement, onStateChange, onScreenTrack } = options;
        if (!token) throw new LiveRelayError('AUTH_ERROR', 'token is required');
        if (!element) throw new LiveRelayError('MEDIA_ERROR', 'element is required');

        // Ensure the video element is ready for autoplay.
        element.autoplay = true;
        element.playsInline = true;

        // Fetch TURN/STUN credentials from the server (with fallback).
        const iceServers = await fetchIceServers(this.server, token);

        const pc = new RTCPeerConnection({ iceServers });
        const session = new SubscribeSession(pc, onStateChange);

        try {
            // Add recvonly transceivers for video, audio, and potentially screen.
            pc.addTransceiver('video', { direction: 'recvonly' });
            pc.addTransceiver('audio', { direction: 'recvonly' });
            // Third transceiver for screen (server may or may not send it).
            pc.addTransceiver('video', { direction: 'recvonly' });

            // Attach incoming tracks to the appropriate video element.
            // The server uses different stream IDs:
            //   "liverelay-cam"    for camera
            //   "liverelay-screen" for screen share
            pc.ontrack = (event) => {
                const streamId = event.streams?.[0]?.id || '';

                if (streamId.includes('screen')) {
                    // Screen share track.
                    const screenStream = event.streams[0] || new MediaStream([event.track]);
                    session.screenStream = screenStream;

                    if (screenElement) {
                        screenElement.autoplay = true;
                        screenElement.playsInline = true;
                        screenElement.srcObject = screenStream;
                    }
                    if (onScreenTrack) {
                        onScreenTrack(screenStream);
                    }
                } else {
                    // Camera track.
                    if (event.streams && event.streams[0]) {
                        session.cameraStream = event.streams[0];
                        element.srcObject = event.streams[0];
                    } else {
                        if (!element.srcObject) {
                            element.srcObject = new MediaStream();
                        }
                        element.srcObject.addTrack(event.track);
                        session.cameraStream = element.srcObject;
                    }
                }
            };

            // Create offer and set local description.
            const offer = await pc.createOffer();
            await pc.setLocalDescription(offer);

            // Wait until ICE candidates are gathered.
            await waitForIceGathering(pc);

            // Exchange SDP with the server.
            const answer = await postSDP(
                `${this.server}/sfu/subscribe`,
                token,
                pc.localDescription
            );

            await pc.setRemoteDescription(new RTCSessionDescription(answer));
            session._setConnected();
        } catch (err) {
            pc.close();
            session._state = 'failed';
            if (onStateChange) onStateChange('failed');
            throw err;
        }

        return session;
    }

    /**
     * Start a 1:1 call (publish + subscribe in one call).
     *
     * @param {Object} options
     * @param {string} options.token - JWT token with role "call"
     * @param {MediaStream} [options.stream] - Local MediaStream
     * @param {boolean} [options.video] - Request video via getUserMedia
     * @param {boolean} [options.audio] - Request audio via getUserMedia
     * @param {HTMLVideoElement} options.element - Video element for remote stream
     * @param {HTMLVideoElement} [options.localElement] - Video element for local preview
     * @param {function} [options.onStateChange] - Callback(state: string)
     * @returns {Promise<CallSession>}
     */
    async call(options) {
        let { token, stream, video, audio, element, localElement, onStateChange } = options;
        if (!token) throw new LiveRelayError('AUTH_ERROR', 'token is required');
        if (!element) throw new LiveRelayError('MEDIA_ERROR', 'element is required');

        // Auto-capture if no stream provided
        if (!stream && (video || audio)) {
            try {
                stream = await navigator.mediaDevices.getUserMedia({
                    video: video || false,
                    audio: audio || false
                });
            } catch (e) {
                throw new LiveRelayError('MEDIA_ERROR',
                    e.name === 'NotAllowedError'
                        ? 'Camera/microphone permission denied. Please allow access in your browser settings.'
                        : `Media capture failed: ${e.message}`
                );
            }
        }
        if (!stream) throw new LiveRelayError('MEDIA_ERROR', 'stream or video/audio options required');

        // Prepare the remote video element.
        element.autoplay = true;
        element.playsInline = true;

        // Show local preview if a local element is provided.
        if (localElement) {
            localElement.autoplay = true;
            localElement.playsInline = true;
            localElement.muted = true;
            localElement.srcObject = stream;
        }

        // Fetch TURN/STUN credentials from the server (with fallback).
        const iceServers = await fetchIceServers(this.server, token);

        const pc = new RTCPeerConnection({ iceServers });
        const session = new CallSession(pc, onStateChange);

        try {
            // Add all local tracks as sendrecv.
            const addedKinds = new Set();
            for (const track of stream.getTracks()) {
                pc.addTrack(track, stream);
                addedKinds.add(track.kind);
            }

            // Ensure we can receive media kinds we are not already sending.
            if (!addedKinds.has('video')) {
                pc.addTransceiver('video', { direction: 'recvonly' });
            }
            if (!addedKinds.has('audio')) {
                pc.addTransceiver('audio', { direction: 'recvonly' });
            }

            // Attach incoming remote tracks to the video element.
            pc.ontrack = (event) => {
                if (event.streams && event.streams[0]) {
                    element.srcObject = event.streams[0];
                } else {
                    if (!element.srcObject) {
                        element.srcObject = new MediaStream();
                    }
                    element.srcObject.addTrack(event.track);
                }
            };

            // Create offer and set local description.
            const offer = await pc.createOffer();
            await pc.setLocalDescription(offer);

            // Wait until ICE candidates are gathered.
            await waitForIceGathering(pc);

            // Exchange SDP with the server.
            const answer = await postSDP(
                `${this.server}/sfu/call`,
                token,
                pc.localDescription
            );

            await pc.setRemoteDescription(new RTCSessionDescription(answer));
            session._setConnected();
        } catch (err) {
            pc.close();
            session._state = 'failed';
            if (onStateChange) onStateChange('failed');
            throw err;
        }

        return session;
    }

    /**
     * Join an N-party conference room (publish + subscribe to all).
     *
     * Returns a ConferenceSession that:
     *   - Publishes your camera/mic.
     *   - Subscribes to all participants already present.
     *   - Has a `subscribeToParticipant(peerId, element)` method for
     *     subscribing to newcomers who join later.
     *
     * @param {Object} options
     * @param {string} options.token - JWT token with role "conference"
     * @param {MediaStream} [options.stream] - Local MediaStream
     * @param {boolean} [options.video] - Request video via getUserMedia
     * @param {boolean} [options.audio] - Request audio via getUserMedia
     * @param {HTMLVideoElement} [options.localElement] - Video element for local preview
     * @param {function} [options.onTrack] - Callback(peerId, stream) for each remote participant
     * @param {function} [options.onStateChange] - Callback(state: string)
     * @returns {Promise<ConferenceSession>}
     */
    async conference(options) {
        let { token, stream, video, audio, localElement, onTrack, onStateChange } = options;
        if (!token) throw new LiveRelayError('AUTH_ERROR', 'token is required');

        // Auto-capture if no stream provided.
        if (!stream && (video || audio)) {
            try {
                stream = await navigator.mediaDevices.getUserMedia({
                    video: video || false,
                    audio: audio || false
                });
            } catch (e) {
                throw new LiveRelayError('MEDIA_ERROR',
                    e.name === 'NotAllowedError'
                        ? 'Camera/microphone permission denied.'
                        : `Media capture failed: ${e.message}`
                );
            }
        }
        if (!stream) throw new LiveRelayError('MEDIA_ERROR', 'stream or video/audio options required');

        // Show local preview.
        if (localElement) {
            localElement.autoplay = true;
            localElement.playsInline = true;
            localElement.muted = true;
            localElement.srcObject = stream;
        }

        const iceServers = await fetchIceServers(this.server, token);
        const pc = new RTCPeerConnection({ iceServers });

        // Temporary placeholder for conference data.
        const session = new ConferenceSession(pc, onStateChange, { peer_id: '', participants: [] }, this, token);

        try {
            // Add all local tracks (publish).
            const addedKinds = new Set();
            for (const track of stream.getTracks()) {
                pc.addTrack(track, stream);
                addedKinds.add(track.kind);
            }

            // Ensure we can receive all media kinds.
            if (!addedKinds.has('video')) {
                pc.addTransceiver('video', { direction: 'recvonly' });
            }
            if (!addedKinds.has('audio')) {
                pc.addTransceiver('audio', { direction: 'recvonly' });
            }

            // Handle incoming remote tracks.
            // The server uses stream IDs like "lr-{short_peer_id}" to identify
            // which participant a track belongs to.
            pc.ontrack = (event) => {
                const streamId = event.streams?.[0]?.id || 'unknown';
                const remoteStream = event.streams?.[0] || new MediaStream([event.track]);

                // Extract peer_id from stream ID (format: "lr-XXXXXXXX").
                const peerId = streamId.startsWith('lr-') ? streamId : streamId;
                session.remoteStreams.set(peerId, remoteStream);

                if (onTrack) {
                    onTrack(peerId, remoteStream);
                }
            };

            // Create offer and set local description.
            const offer = await pc.createOffer();
            await pc.setLocalDescription(offer);
            await waitForIceGathering(pc);

            // Exchange SDP with the server.
            const answer = await postSDP(
                `${this.server}/sfu/conference`,
                token,
                pc.localDescription
            );

            await pc.setRemoteDescription(new RTCSessionDescription(answer));

            // Update session with conference data from the server.
            session.peerId = answer.peer_id || '';
            session.participants = answer.participants || [];

            session._setConnected();
        } catch (err) {
            pc.close();
            session._state = 'failed';
            if (onStateChange) onStateChange('failed');
            throw err;
        }

        return session;
    }
}

// ---------------------------------------------------------------------------
// Version
// ---------------------------------------------------------------------------

LiveRelay.VERSION = '0.4.0';

// ---------------------------------------------------------------------------
// Export: ES module + global fallback for <script> tag usage
// ---------------------------------------------------------------------------

export {
    LiveRelay,
    PublishSession,
    SubscribeSession,
    CallSession,
    ConferenceSession,
    LiveRelayError,
    BaseSession
};
export default LiveRelay;

if (typeof window !== 'undefined') {
    window.LiveRelay = LiveRelay;
    window.LiveRelayError = LiveRelayError;
    window.ConferenceSession = ConferenceSession;
}
