import React, { useState, useEffect, useCallback, useRef } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { useAuth } from '@/contexts/AuthContext';
import io from 'socket.io-client';
import { toast } from 'sonner';
import AccountSettings from '@/components/AccountSettings';

const API_URL = process.env.REACT_APP_BACKEND_URL;

const SFU_URL = process.env.REACT_APP_SFU_URL;

export const Space = () => {
  const navigate = useNavigate();
  const location = useLocation();
  const { user } = useAuth();
  const mediaInputRef = useRef(null);
  const socketRef = useRef(null);
  const messagesEndRef = useRef(null);
  
  // Notifications state
  const [showNotif, setShowNotif] = useState(false);
  const [unreadCount, setUnreadCount] = useState(0);
  const [notifications, setNotifications] = useState([]);
  
  // Account menu state
  const [showAccountMenu, setShowAccountMenu] = useState(false);
  
  // Following list (users we follow)
  const [following, setFollowing] = useState([]);
  const [loadingFollowing, setLoadingFollowing] = useState(true);
  
  // Messages list
  const [messages, setMessages] = useState([]);
  const [loadingMessages, setLoadingMessages] = useState(true);
  
  // Full followers view
  const [showAllFollowers, setShowAllFollowers] = useState(false);
  const [followerSearchQuery, setFollowerSearchQuery] = useState('');
  
  // Follower profile modal
  const [selectedFollower, setSelectedFollower] = useState(null);
  
  // Group management modal
  const [showGroupModal, setShowGroupModal] = useState(false);
  const [groupMembers, setGroupMembers] = useState([]);
  const [groupName, setGroupName] = useState('');
  const [isEditingGroupName, setIsEditingGroupName] = useState(false);
  const [newGroupName, setNewGroupName] = useState('');
  
  // New message modal
  const [showNewMessage, setShowNewMessage] = useState(false);
  const [selectedContacts, setSelectedContacts] = useState([]);
  const [contactSearchQuery, setContactSearchQuery] = useState('');
  
  // Conversation view
  const [activeConversation, setActiveConversation] = useState(null);
  const activeConversationRef = useRef(null);
  const [conversationMessages, setConversationMessages] = useState([]);
  const [newMessageText, setNewMessageText] = useState('');
  
  // Video call states
  const [callState, setCallState] = useState(null); // null | 'outgoing' | 'incoming' | 'active'
  const [callData, setCallData] = useState(null);
  const [localStream, setLocalStream] = useState(null);
  const [remoteStream, setRemoteStream] = useState(null);
  const peerConnectionRef = useRef(null);   // publish PC
  const subscribePcRef = useRef(null);      // subscribe PC (receives other peer's tracks)
  const sfuCleanedUpRef = useRef(false);    // guard for subscribe retry loop
  const [callDuration, setCallDuration] = useState(0);
  const callTimerRef = useRef(null);
  const [isMuted, setIsMuted] = useState(false);
  const [isCameraOff, setIsCameraOff] = useState(false);
  const localVideoRef = useRef(null);
  const remoteVideoRef = useRef(null);
  
  // Active call partner enriched info
  const [callPartnerProfile, setCallPartnerProfile] = useState(null);
  const [callPartnerLiked, setCallPartnerLiked] = useState(false);
  const [callLikesReceived, setCallLikesReceived] = useState(0);
  const [callFollowStatus, setCallFollowStatus] = useState(null); // null, 'pending', 'accepted'
  const [showCallGiftPopup, setShowCallGiftPopup] = useState(false);
  const [showCallKinksOverlay, setShowCallKinksOverlay] = useState(false);
  const [showCallChat, setShowCallChat] = useState(false);
  const [callChatText, setCallChatText] = useState('');

  // Report message overlay
  const [showReportOverlay, setShowReportOverlay] = useState(false);
  const [reportTarget, setReportTarget] = useState(null); // { type: 'message' | 'profile', id, user_id, user_name }
  const [showReportConfirm, setShowReportConfirm] = useState(false);
  const [reportStep, setReportStep] = useState(1);
  const [reportReason, setReportReason] = useState('');
  const [reportComment, setReportComment] = useState('');
  
  // Remove from Bros confirmation
  const [showRemoveBroConfirm, setShowRemoveBroConfirm] = useState(false);
  const [broToRemove, setBroToRemove] = useState(null);
  
  // Message unread count for nav badge
  const [messageUnreadCount, setMessageUnreadCount] = useState(0);
  // Revealed private messages (Set of message IDs)
  const [revealedMessages, setRevealedMessages] = useState(new Set());
  // Pending media for modal choice (normal/private)
  const [pendingMedia, setPendingMedia] = useState(null); // { file, isImage, isVideo, previewUrl }
  const [showMediaModal, setShowMediaModal] = useState(false);
  // Account settings modal
  const [showAccountSettings, setShowAccountSettings] = useState(false);
  // Single view toggle (UI only)
  const [singleViewEnabled, setSingleViewEnabled] = useState(false);
  // Add Bro modal
  const [showAddBro, setShowAddBro] = useState(false);
  const [addBroQuery, setAddBroQuery] = useState('');
  const [addBroResults, setAddBroResults] = useState([]);
  const [addBroLoading, setAddBroLoading] = useState(false);
  const [pendingFollows, setPendingFollows] = useState(new Set());
  const addBroDebounceRef = useRef(null);
  // Fetch unread notification count
  const fetchUnreadCount = useCallback(async () => {
    try {
      const token = localStorage.getItem('brozr_token');
      if (!token) return;
      
      const res = await fetch(`${API_URL}/api/notifications/unread-count`, {
        headers: { 'Authorization': `Bearer ${token}` }
      });
      if (res.ok) {
        const json = await res.json();
        const data = json.data || json;
        setUnreadCount(data.count || 0);
      }
    } catch (err) {
      console.error('Failed to fetch unread count:', err);
    }
  }, []);

  // Fetch notifications list
  const fetchNotifications = useCallback(async () => {
    try {
      const token = localStorage.getItem('brozr_token');
      if (!token) return;
      
      const res = await fetch(`${API_URL}/api/notifications`, {
        headers: { 'Authorization': `Bearer ${token}` }
      });
      if (res.ok) {
        const json = await res.json();
        const notifs = json.data || json;
        setNotifications(Array.isArray(notifs) ? notifs : []);
      }
    } catch (err) {
      console.error('Failed to fetch notifications:', err);
    }
  }, []);

  // Mark all notifications as read
  const markAllAsRead = useCallback(async () => {
    try {
      const token = localStorage.getItem('brozr_token');
      if (!token) return;
      
      await fetch(`${API_URL}/api/notifications/mark-all-read`, {
        method: 'POST',
        headers: { 'Authorization': `Bearer ${token}` }
      });
      setUnreadCount(0);
    } catch (err) {
      console.error('Failed to mark notifications as read:', err);
    }
  }, []);

  // Fetch following list (all bros with accepted follow relationship)
  const fetchFollowing = useCallback(async () => {
    try {
      const token = localStorage.getItem('brozr_token');
      if (!token) return;

      const res = await fetch(`${API_URL}/api/users/following`, {
        headers: { 'Authorization': `Bearer ${token}` }
      });
      if (res.ok) {
        const json = await res.json();
        const bros = json.data || json;
        setFollowing(Array.isArray(bros) ? bros : []);
      }
    } catch (err) {
      console.error('Failed to fetch following:', err);
    } finally {
      setLoadingFollowing(false);
    }
  }, []);

  // Fetch messages
  const fetchMessages = useCallback(async () => {
    try {
      const token = localStorage.getItem('brozr_token');
      if (!token) return;
      
      const res = await fetch(`${API_URL}/api/messages/conversations`, {
        headers: { 'Authorization': `Bearer ${token}` }
      });
      if (res.ok) {
        const json = await res.json();
        const rawConvs = json.data || json;
        // Normalize backend field names for frontend usage
        const convs = (Array.isArray(rawConvs) ? rawConvs : []).map(c => ({
          ...c,
          group_photo: c.group_photo_url || c.group_photo || null,
        }));
        setMessages(convs);
      }
    } catch (err) {
      console.error('Failed to fetch messages:', err);
      setMessages([]);
    } finally {
      setLoadingMessages(false);
    }
  }, []);

  // Fetch group members
  // Group photo state
  const [groupPhoto, setGroupPhoto] = useState(null);
  const groupPhotoInputRef = useRef(null);
  
  const fetchGroupMembers = useCallback(async (conversationId) => {
    try {
      const token = localStorage.getItem('brozr_token');
      if (!token) return;
      
      const res = await fetch(`${API_URL}/api/messages/conversations/${conversationId}`, {
        headers: { 'Authorization': `Bearer ${token}` }
      });
      if (res.ok) {
        const json = await res.json();
        const data = json.data || json;
        setGroupMembers(data.members || []);
        setGroupName(data.group_name || data.name || '');
        setNewGroupName(data.group_name || data.name || '');
        setGroupPhoto(data.group_photo_url || data.group_photo || null);
      }
    } catch (err) {
      console.error('Failed to fetch group members:', err);
    }
  }, []);

  // Upload group photo
  const handleGroupPhotoUpload = useCallback(async (e) => {
    const file = e.target.files?.[0];
    if (!file || !activeConversation?.id) return;
    
    try {
      const token = localStorage.getItem('brozr_token');
      if (!token) return;
      
      const formData = new FormData();
      formData.append('file', file);
      
      const res = await fetch(`${API_URL}/api/messages/conversations/group/${activeConversation.id}/photo`, {
        method: 'POST',
        headers: { 'Authorization': `Bearer ${token}` },
        body: formData
      });
      
      if (res.ok) {
        const json = await res.json();
        const data = json.data || json;
        setGroupPhoto(data.group_photo);
        // Update conversation in list
        setMessages(prev => prev.map(conv => 
          conv.id === activeConversation.id 
            ? { ...conv, group_photo: data.group_photo }
            : conv
        ));
        setActiveConversation(prev => ({ ...prev, group_photo: data.group_photo }));
      }
    } catch (err) {
      console.error('Failed to upload group photo:', err);
    }
  }, [activeConversation?.id]);

  // Rename group
  const handleRenameGroup = useCallback(async () => {
    if (!activeConversation?.id || !newGroupName.trim()) return;
    
    try {
      const token = localStorage.getItem('brozr_token');
      if (!token) return;
      
      const res = await fetch(`${API_URL}/api/messages/conversations/group/${activeConversation.id}/name`, {
        method: 'PUT',
        headers: { 
          'Authorization': `Bearer ${token}`,
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({ name: newGroupName.trim() })
      });
      
      if (res.ok) {
        setGroupName(newGroupName.trim());
        setIsEditingGroupName(false);
        // Update conversation in list
        setMessages(prev => prev.map(conv =>
          conv.id === activeConversation.id
            ? { ...conv, group_name: newGroupName.trim(), name: newGroupName.trim() }
            : conv
        ));
        // Update active conversation
        setActiveConversation(prev => ({ ...prev, group_name: newGroupName.trim(), name: newGroupName.trim() }));
      }
    } catch (err) {
      console.error('Failed to rename group:', err);
    }
  }, [activeConversation?.id, newGroupName]);

  // Initial data fetch + refresh when page becomes visible
  useEffect(() => {
    fetchUnreadCount();
    fetchFollowing();
    fetchMessages();

    const interval = setInterval(fetchUnreadCount, 60000);
    
    // Refresh data when user comes back to this tab/page
    const handleVisibilityChange = () => {
      if (document.visibilityState === 'visible') {
        fetchFollowing();
        fetchMessages();
      }
    };
    
    const handleFocus = () => {
      fetchFollowing();
      fetchMessages();
    };
    
    document.addEventListener('visibilitychange', handleVisibilityChange);
    window.addEventListener('focus', handleFocus);
    
    return () => {
      clearInterval(interval);
      document.removeEventListener('visibilitychange', handleVisibilityChange);
      window.removeEventListener('focus', handleFocus);
    };
  }, [fetchUnreadCount, fetchFollowing, fetchMessages]);

  // Keep ref in sync with activeConversation for socket handlers (avoid stale closure)
  useEffect(() => {
    activeConversationRef.current = activeConversation;
  }, [activeConversation]);

  // Socket.IO connection for real-time messages and live cam requests
  useEffect(() => {
    const token = localStorage.getItem('brozr_token');
    if (!token) return;

    // Only create socket if not already connected
    if (socketRef.current?.connected) return;

    // Create socket connection to messaging service
    const MESSAGING_SOCKET_URL = process.env.REACT_APP_MESSAGING_SOCKET_URL;
    const socket = io(MESSAGING_SOCKET_URL, {
      auth: { token },
      query: { token },
      transports: ['websocket', 'polling'],
      reconnection: true,
      reconnectionAttempts: 5,
      reconnectionDelay: 1000
    });
    socketRef.current = socket;
    
    // Listen for new messages
    socket.on('new_message', (data) => {
      // Update messages list (conversations) with new message preview
      setMessages(prev => {
        // Check if conversation already exists
        const existingIndex = prev.findIndex(conv => conv.id === data.conversation_id);
        
        if (existingIndex >= 0) {
          // Update existing conversation
          const updated = [...prev];
          updated[existingIndex] = {
            ...updated[existingIndex],
            last_message: data.message?.content || updated[existingIndex].last_message,
            last_message_time: data.message?.created_at || new Date().toISOString(),
            unread_count: (updated[existingIndex].unread_count || 0) + (data.message?.sender_id !== (user?.credential_id || user?.id) ? 1 : 0)
          };
          // Sort by last_message_time
          return updated.sort((a, b) => new Date(b.last_message_time) - new Date(a.last_message_time));
        } else {
          // New conversation - add to list with group info if available
          const newConv = {
            id: data.conversation_id,
            partner_id: data.message?.sender_id,
            partner_name: data.sender_name || 'User',
            partner_photo: data.sender_photo,
            last_message: data.message?.content,
            last_message_time: data.message?.created_at || new Date().toISOString(),
            unread_count: 1,
            is_group: data.is_group || false,
            group_name: data.group_name || null,
            name: data.group_name || null,
          };
          // For new group conversations, fetch full conversation list to get proper data
          if (data.is_group) {
            fetchMessages();
          }
          return [newConv, ...prev];
        }
      });
      
      // If we're in the conversation that received the message, add it to messages
      // Use ref to avoid stale closure (activeConversation would be null from mount time)
      const currentConv = activeConversationRef.current;
      if (currentConv && data.conversation_id === currentConv.id) {
        setConversationMessages(prev => {
          // Don't add duplicates
          if (prev.some(m => m.id === data.message?.id)) return prev;
          return [...prev, data.message];
        });
        // Mark as read immediately since we're viewing
        setMessages(prev => prev.map(conv =>
          conv.id === data.conversation_id ? { ...conv, unread_count: 0 } : conv
        ));
        // Mark as read on server too
        const token = localStorage.getItem('brozr_token');
        if (token) {
          fetch(`${API_URL}/api/messages/conversations/${data.conversation_id}/read`, {
            method: 'POST',
            headers: { 'Authorization': `Bearer ${token}` }
          }).catch(() => {});
        }
        // Auto-scroll to bottom
        setTimeout(() => {
          messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
        }, 100);
      }
      
      // Update notification count
      fetchUnreadCount();
    });
    
    // Listen for group renamed events
    socket.on('group-renamed', (data) => {
      // Update the conversation in the list
      setMessages(prev => prev.map(conv =>
        conv.id === data.conversation_id ? { ...conv, group_name: data.name, name: data.name } : conv
      ));
      // Update active conversation if it's the one that was renamed
      if (activeConversation && activeConversation.id === data.conversation_id) {
        setActiveConversation(prev => ({ ...prev, group_name: data.name, name: data.name }));
        setGroupName(data.name);
        setNewGroupName(data.name);
      }
    });
    
    // Listen for group photo updated events
    socket.on('group-photo-updated', (data) => {
      // Update the conversation in the list
      setMessages(prev => prev.map(conv => 
        conv.id === data.conversation_id ? { ...conv, group_photo: data.group_photo } : conv
      ));
      // Update active conversation if it's the one that was updated
      if (activeConversation && activeConversation.id === data.conversation_id) {
        setActiveConversation(prev => ({ ...prev, group_photo: data.group_photo }));
        setGroupPhoto(data.group_photo);
      }
    });
    
    // Listen for call events
    socket.on('call-created', (data) => {
      setCallData(prev => prev ? { ...prev, call_id: data.call_id, room_id: data.room_id, sfu_token: data.sfu_token } : prev);
    });

    socket.on('incoming-call', (data) => {
      // Check if caller is ignored (60 min TTL)
      try {
        const ignored = JSON.parse(localStorage.getItem('brozr_ignored_users') || '{}');
        const ts = ignored[data.caller_id];
        if (ts && Date.now() - ts < 60 * 60 * 1000) {
          socket.emit('call-decline', { call_id: data.call_id });
          return;
        }
      } catch (e) {}
      setCallState('incoming');
      setCallData({
        call_id: data.call_id,
        room_id: data.room_id,
        sfu_token: data.sfu_token,
        caller_id: data.caller_id,
        caller_name: data.caller_name,
        caller_photo: data.caller_photo,
      });
    });

    socket.on('call-accepted', (data) => {
      setCallState('active');
      setCallData(prev => prev ? { ...prev, call_id: data.call_id } : prev);
    });

    socket.on('call-declined', (data) => {
      toast.error('Appel refusé');
      // Cleanup - stop local stream that was acquired in startCall
      setLocalStream(prev => { if (prev) prev.getTracks().forEach(t => t.stop()); return null; });
      setCallState(null);
      setCallData(null);
    });

    socket.on('call-ended', (data) => {
      toast('Appel terminé');
      sfuCleanedUpRef.current = true;
      if (peerConnectionRef.current) {
        peerConnectionRef.current.close();
        peerConnectionRef.current = null;
      }
      if (subscribePcRef.current) {
        subscribePcRef.current.close();
        subscribePcRef.current = null;
      }
      // Clear video srcObjects to avoid stale streams
      if (remoteVideoRef.current) remoteVideoRef.current.srcObject = null;
      if (localVideoRef.current) localVideoRef.current.srcObject = null;
      setCallState(null);
      setCallData(null);
      setLocalStream(prev => { if (prev) prev.getTracks().forEach(t => t.stop()); return null; });
      setRemoteStream(null);
      setCallDuration(0);
      setIsMuted(false);
      setIsCameraOff(false);
    });
    
    // Listen for bro removal (when someone unfollows us)
    socket.on('bro-removed', (data) => {
      // Remove from following list immediately
      setFollowing(prev => prev.filter(f => f.id !== data.removed_by));
    });
    
    // Listen for notification updates
    socket.on('notification-read', () => {
      fetchUnreadCount();
    });
    
    // Listen for real-time presence updates
    socket.on('user-online', (data) => {
      setFollowing(prev => prev.map(f =>
        f.credential_id === data.user_id ? { ...f, is_online: true } : f
      ));
      setMessages(prev => prev.map(conv =>
        conv.partner_id === data.user_id ? { ...conv, partner_online: true } : conv
      ));
    });

    socket.on('user-offline', (data) => {
      setFollowing(prev => prev.map(f =>
        f.credential_id === data.user_id ? { ...f, is_online: false } : f
      ));
      setMessages(prev => prev.map(conv =>
        conv.partner_id === data.user_id ? { ...conv, partner_online: false } : conv
      ));
    });

    socket.on('connect', () => {});
    socket.on('disconnect', () => {});
    socket.on('reconnect', () => {});

    // Heartbeat to keep presence alive (every 50s, TTL is 120s)
    const heartbeatInterval = setInterval(() => {
      if (socket.connected) {
        socket.emit('heartbeat');
      }
    }, 50000);

    return () => {
      clearInterval(heartbeatInterval);
      socket.disconnect();
      socketRef.current = null;
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []); // Empty deps - only run once on mount
  
  // Separate effect to handle activeConversation changes for messages
  useEffect(() => {
    if (!activeConversation || !socketRef.current) return;
    
    // When opening a conversation, mark messages as read
    const markAsRead = async () => {
      if (activeConversation.unread_count > 0) {
        // Update local state immediately
        setMessages(prev => prev.map(conv => 
          conv.id === activeConversation.id ? { ...conv, unread_count: 0 } : conv
        ));
        // Fetch will also mark as read on server
        fetchUnreadCount();
      }
    };
    markAsRead();
  }, [activeConversation?.id, fetchUnreadCount]);

  // Handle notification bell click
  const handleNotifClick = async () => {
    const wasOpen = showNotif;
    setShowNotif(!showNotif);
    setShowAccountMenu(false);
    
    if (!wasOpen) {
      await fetchNotifications();
      if (unreadCount > 0) {
        await markAllAsRead();
      }
    }
  };

  // Handle account menu click
  const handleAccountMenuClick = () => {
    setShowAccountMenu(!showAccountMenu);
    setShowNotif(false);
  };

  // Handle follower click - open profile modal
  const handleFollowerClick = (followedUser) => {
    setSelectedFollower(followedUser);
  };

  // Handle message button in follower profile
  const handleStartConversation = (follower) => {
    setSelectedFollower(null);
    setActiveConversation({
      id: `temp_${follower.id}`,
      partner_id: follower.id,
      partner_name: follower.display_name,
      partner_photo: follower.profile_photo,
      partner_online: follower.is_online
    });
    setConversationMessages([]);
  };

  // ===== VIDEO CALL FUNCTIONS =====
  const cleanupCall = useCallback(() => {
    sfuCleanedUpRef.current = true;
    if (peerConnectionRef.current) {
      peerConnectionRef.current.close();
      peerConnectionRef.current = null;
    }
    if (subscribePcRef.current) {
      subscribePcRef.current.close();
      subscribePcRef.current = null;
    }
    if (localStream) {
      localStream.getTracks().forEach(t => t.stop());
    }
    // Clear video element srcObjects to avoid stale streams on next call
    if (remoteVideoRef.current) {
      remoteVideoRef.current.srcObject = null;
    }
    if (localVideoRef.current) {
      localVideoRef.current.srcObject = null;
    }
    if (callTimerRef.current) {
      clearInterval(callTimerRef.current);
      callTimerRef.current = null;
    }
    setCallState(null);
    setCallData(null);
    setLocalStream(null);
    setRemoteStream(null);
    setCallDuration(0);
    setIsMuted(false);
    setIsCameraOff(false);
  }, [localStream]);

  // Helper: create offer + wait for ICE gathering to complete
  const createGatheredOffer = async (pc) => {
    const offer = await pc.createOffer();
    await pc.setLocalDescription(offer);
    await new Promise((resolve) => {
      if (pc.iceGatheringState === 'complete') return resolve();
      pc.onicegatheringstatechange = () => {
        if (pc.iceGatheringState === 'complete') resolve();
      };
    });
    return pc.localDescription;
  };

  const connectToSFU = useCallback(async (roomId, sfuToken, stream) => {
    sfuCleanedUpRef.current = false;

    // 1. Get ICE servers from SFU
    const iceRes = await fetch(`${SFU_URL}/v1/ice-servers`, {
      headers: { 'Authorization': `Bearer ${sfuToken}` }
    });
    const iceConfig = await iceRes.json();

    // === PUBLISH: send our tracks to SFU ===
    const pubPc = new RTCPeerConnection({ iceServers: iceConfig.iceServers });
    peerConnectionRef.current = pubPc;

    stream.getTracks().forEach(track => pubPc.addTrack(track, stream));

    pubPc.oniceconnectionstatechange = () => {};

    const pubDesc = await createGatheredOffer(pubPc);
    const pubRes = await fetch(`${SFU_URL}/sfu/publish`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${sfuToken}`
      },
      body: JSON.stringify({ sdp: pubDesc.sdp, type: 'offer' })
    });
    if (!pubRes.ok) throw new Error(`Publish failed: ${pubRes.status}`);
    if (sfuCleanedUpRef.current) { pubPc.close(); return; }
    const pubAnswer = await pubRes.json();
    await pubPc.setRemoteDescription(new RTCSessionDescription({ type: 'answer', sdp: pubAnswer.sdp }));
    if (sfuCleanedUpRef.current) { pubPc.close(); peerConnectionRef.current = null; return; }

    // === SUBSCRIBE: receive other peer's tracks (retry until they publish) ===
    const maxAttempts = 20;
    for (let attempt = 0; attempt < maxAttempts; attempt++) {
      if (sfuCleanedUpRef.current) return; // call was ended during retry

      try {
        const subPc = new RTCPeerConnection({ iceServers: iceConfig.iceServers });
        subPc.addTransceiver('video', { direction: 'recvonly' });
        subPc.addTransceiver('audio', { direction: 'recvonly' });

        // Always create a fresh MediaStream for this subscribe attempt
        const remoteMs = new MediaStream();

        subPc.ontrack = (e) => {
          remoteMs.addTrack(e.track);
          // Assign to video element and play
          if (remoteVideoRef.current) {
            remoteVideoRef.current.srcObject = remoteMs;
            remoteVideoRef.current.play().catch(() => {});
          }
        };

        subPc.oniceconnectionstatechange = () => {};

        const subDesc = await createGatheredOffer(subPc);
        const subRes = await fetch(`${SFU_URL}/sfu/subscribe`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${sfuToken}`
          },
          body: JSON.stringify({ sdp: subDesc.sdp, type: 'offer' })
        });

        // Check cleanup flag after async fetch (call might have ended while waiting)
        if (sfuCleanedUpRef.current) {
          subPc.close();
          return;
        }

        if (subRes.ok) {
          const subAnswer = await subRes.json();
          await subPc.setRemoteDescription(new RTCSessionDescription({ type: 'answer', sdp: subAnswer.sdp }));
          subscribePcRef.current = subPc;
          setRemoteStream(remoteMs);
          return;
        } else {
          subPc.close();
          await new Promise(r => setTimeout(r, 1500));
        }
      } catch (err) {
        await new Promise(r => setTimeout(r, 1500));
      }
    }
  }, []);

  const startCall = useCallback(async (partnerId, partnerName, partnerPhoto, isOnline) => {
    // Check if user is online before initiating call
    if (isOnline === false) {
      toast.error('Utilisateur hors ligne');
      return;
    }
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ video: true, audio: true });
      setLocalStream(stream);
      setCallState('outgoing');
      setCallData({ partner_id: partnerId, partner_name: partnerName, partner_photo: partnerPhoto });

      if (socketRef.current) {
        socketRef.current.emit('call-invite', {
          to: partnerId,
          caller_name: user?.display_name || 'User',
          caller_photo: user?.profile_photo || null,
        });
      }
    } catch (err) {
      console.error('[Space] Failed to get media:', err);
      toast.error('Impossible d\'accéder à la caméra/micro');
    }
  }, [user]);

  const acceptCall = useCallback(async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ video: true, audio: true });
      setLocalStream(stream);

      if (socketRef.current && callData?.call_id) {
        socketRef.current.emit('call-accept', { call_id: callData.call_id });
      }
    } catch (err) {
      console.error('[Space] Failed to get media:', err);
      toast.error('Impossible d\'accéder à la caméra/micro');
    }
  }, [callData]);

  const declineCall = useCallback(() => {
    if (socketRef.current && callData?.call_id) {
      socketRef.current.emit('call-decline', { call_id: callData.call_id });
    }
    setCallState(null);
    setCallData(null);
  }, [callData]);

  const handleIgnoreCall = useCallback(() => {
    if (!callData) return;
    // Store caller in ignored list with timestamp (60 min TTL)
    try {
      const ignored = JSON.parse(localStorage.getItem('brozr_ignored_users') || '{}');
      ignored[callData.caller_id] = Date.now();
      localStorage.setItem('brozr_ignored_users', JSON.stringify(ignored));
    } catch (e) { /* ignore */ }
    // Decline the call silently
    if (socketRef.current && callData.call_id) {
      socketRef.current.emit('call-decline', { call_id: callData.call_id });
    }
    setCallState(null);
    setCallData(null);
  }, [callData]);

  const endCall = useCallback(() => {
    if (socketRef.current && callData?.call_id) {
      socketRef.current.emit('call-end', { call_id: callData.call_id });
    }
    cleanupCall();
  }, [callData, cleanupCall]);

  // Connect to SFU when call becomes active AND localStream is ready
  useEffect(() => {
    if (callState === 'active' && localStream && callData?.sfu_token && !peerConnectionRef.current) {
      connectToSFU(callData.room_id, callData.sfu_token, localStream).catch(err => {
        console.error('[Space] SFU connection failed:', err);
        toast.error("Erreur de connexion vidéo");
        cleanupCall();
      });
    }
  }, [callState, localStream, callData, connectToSFU, cleanupCall]);

  // Attach streams to video elements
  useEffect(() => {
    if (localVideoRef.current && localStream) {
      localVideoRef.current.srcObject = localStream;
    }
  }, [localStream]);

  useEffect(() => {
    if (remoteVideoRef.current && remoteStream && remoteVideoRef.current.srcObject !== remoteStream) {
      remoteVideoRef.current.srcObject = remoteStream;
    }
  }, [remoteStream]);

  // Call duration timer
  useEffect(() => {
    if (callState === 'active') {
      callTimerRef.current = setInterval(() => setCallDuration(d => d + 1), 1000);
    } else {
      if (callTimerRef.current) {
        clearInterval(callTimerRef.current);
        callTimerRef.current = null;
      }
    }
    return () => {
      if (callTimerRef.current) clearInterval(callTimerRef.current);
    };
  }, [callState]);

  // Fetch partner profile when call becomes active
  useEffect(() => {
    if (callState !== 'active' || !callData) return;
    const partnerId = callData.partner_id || callData.caller_id;
    if (!partnerId) return;

    const token = localStorage.getItem('brozr_token');
    if (!token) return;

    setCallPartnerLiked(false);
    setCallLikesReceived(0);
    setCallFollowStatus(null);
    setCallPartnerProfile(null);
    setShowCallChat(false);
    setCallChatText('');

    // Fetch partner profile
    fetch(`${API_URL}/api/users/profile/${partnerId}`, {
      headers: { Authorization: `Bearer ${token}` }
    })
      .then(res => res.json())
      .then(json => {
        const profile = json.data || json;
        setCallPartnerProfile(profile);
        setCallLikesReceived(profile.total_likes || profile.likes_count || 0);
      })
      .catch(() => {});

    // Check if already liked this partner
    fetch(`${API_URL}/api/users/likes/check/${partnerId}`, {
      headers: { Authorization: `Bearer ${token}` }
    })
      .then(res => res.ok ? res.json() : null)
      .then(json => {
        if (json) {
          const d = json.data || json;
          setCallPartnerLiked(!!d.already_liked);
        }
      })
      .catch(() => {});

    // Check if already following this partner
    fetch(`${API_URL}/api/users/following`, {
      headers: { Authorization: `Bearer ${token}` }
    })
      .then(res => res.ok ? res.json() : null)
      .then(json => {
        if (json) {
          const bros = json.data || json;
          const ids = new Set((Array.isArray(bros) ? bros : []).flatMap(b => [b.id, b.credential_id].filter(Boolean)));
          setCallFollowStatus(ids.has(partnerId) ? 'accepted' : null);
        }
      })
      .catch(() => {});
  }, [callState, callData]);

  // Like partner during active call
  const handleCallLike = useCallback(async () => {
    if (callPartnerLiked || !callData) return;
    const partnerId = callData.partner_id || callData.caller_id;
    if (!partnerId) return;

    const token = localStorage.getItem('brozr_token');
    if (!token) return;

    try {
      await fetch(`${API_URL}/api/users/likes`, {
        method: 'POST',
        headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
        body: JSON.stringify({ liked_id: partnerId }),
      });
      setCallPartnerLiked(true);
      setCallLikesReceived(prev => prev + 1);
    } catch (e) {
      // silent
    }
  }, [callPartnerLiked, callData]);

  // Follow partner during active call
  const handleCallFollow = useCallback(async () => {
    if (!callData || callFollowStatus === 'pending' || callFollowStatus === 'accepted') return;
    const partnerId = callData.partner_id || callData.caller_id;
    if (!partnerId) return;

    const token = localStorage.getItem('brozr_token');
    if (!token) return;

    setCallFollowStatus('pending');
    try {
      const res = await fetch(`${API_URL}/api/users/follows/${partnerId}`, {
        method: 'POST',
        headers: { Authorization: `Bearer ${token}` },
      });
      if (res.ok) {
        setCallFollowStatus('accepted');
      } else {
        setCallFollowStatus(null);
      }
    } catch (e) {
      setCallFollowStatus(null);
    }
  }, [callData, callFollowStatus]);

  // Handle incoming call from GlobalLiveCamListener banner (user already clicked Accept)
  // Skip the incoming overlay → auto-accept once socket is ready
  useEffect(() => {
    if (!location.state?.incomingCall || callState) return;

    const call = location.state.incomingCall;
    setCallData({
      call_id: call.call_id,
      room_id: call.room_id,
      sfu_token: call.sfu_token,
      caller_id: call.caller_id,
      caller_name: call.caller_name,
      caller_photo: call.caller_photo,
    });
    window.history.replaceState({}, document.title);

    // Auto-accept: wait for socket to be connected then emit call-accept + get media
    const doAccept = async () => {
      const socket = socketRef.current;
      const emitAccept = () => {
        (async () => {
          try {
            const stream = await navigator.mediaDevices.getUserMedia({ video: true, audio: true });
            setLocalStream(stream);
            socketRef.current.emit('call-accept', { call_id: call.call_id });
          } catch (err) {
            console.error('[Space] Failed to get media for banner accept:', err);
            toast.error("Impossible d'accéder à la caméra/micro");
            setCallState(null);
            setCallData(null);
          }
        })();
      };

      if (socket?.connected) {
        emitAccept();
      } else if (socket) {
        socket.once('connect', emitAccept);
      } else {
        // Socket not yet created — wait a bit and retry
        const interval = setInterval(() => {
          if (socketRef.current?.connected) {
            clearInterval(interval);
            emitAccept();
          }
        }, 100);
        setTimeout(() => clearInterval(interval), 5000);
      }
    };
    doAccept();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [location.state]);

  // Handle new message click
  const handleNewMessage = () => {
    setShowNewMessage(true);
    setSelectedContacts([]);
    setContactSearchQuery('');
  };

  // Toggle contact selection for group message
  const toggleContactSelection = (contact) => {
    if (selectedContacts.find(c => c.id === contact.id)) {
      setSelectedContacts(selectedContacts.filter(c => c.id !== contact.id));
    } else {
      setSelectedContacts([...selectedContacts, contact]);
    }
  };

  // Start conversation with selected contacts
  const startConversation = () => {
    if (selectedContacts.length === 0) return;
    
    if (selectedContacts.length === 1) {
      // 1-on-1 conversation
      setActiveConversation({
        id: `temp_${selectedContacts[0].id}`,
        partner_id: selectedContacts[0].credential_id || selectedContacts[0].id,
        partner_name: selectedContacts[0].display_name,
        partner_photo: selectedContacts[0].profile_photo,
        partner_online: selectedContacts[0].is_online,
        is_group: false
      });
    } else {
      // Group conversation
      setActiveConversation({
        id: `temp_group_${Date.now()}`,
        participants: selectedContacts,
        is_group: true
      });
    }
    setShowNewMessage(false);
    setConversationMessages([]);
  };

  // Send message
  const sendMessage = async () => {
    if (!newMessageText.trim() || !activeConversation) return;
    
    const token = localStorage.getItem('brozr_token');
    if (!token) return;
    
    const content = newMessageText.trim();
    
    // Optimistic update - add message immediately
    const tempMsg = {
      id: `temp_${Date.now()}`,
      sender_id: user?.credential_id || user?.id,
      content: content,
      created_at: new Date().toISOString()
    };
    setConversationMessages([...conversationMessages, tempMsg]);
    setNewMessageText('');
    
    try {
      // Build request body based on conversation type
      const body = {
        conversation_id: activeConversation.id?.startsWith('temp') ? null : activeConversation.id,
        content: content
      };
      
      if (activeConversation.is_group && activeConversation.participants) {
        // Group message - send to all participants
        body.participants = activeConversation.participants.map(p => p.credential_id || p.id);
        body.is_group = true;
      } else {
        // 1-on-1 message
        body.partner_id = activeConversation.partner_id;
      }
      
      const res = await fetch(`${API_URL}/api/messages/send`, {
        method: 'POST',
        headers: { 
          'Authorization': `Bearer ${token}`,
          'Content-Type': 'application/json'
        },
        body: JSON.stringify(body)
      });
      
      if (res.ok) {
        const json = await res.json();
        const data = json.data || json;
        // Replace temp message with real one from API (Leglasense pattern: dedup)
        if (data.message) {
          setConversationMessages(prev => prev.map(m =>
            m.id === tempMsg.id ? data.message : m
          ));
        }
        // Update conversation ID if it was new
        if (activeConversation.id?.startsWith('temp')) {
          setActiveConversation({
            ...activeConversation,
            id: data.conversation_id
          });
        }
        // Refresh conversation list to update last message preview
        fetchMessages();
      } else if (res.status === 403) {
        // Not Bros anymore - show message and mark conversation as read-only
        setActiveConversation(prev => ({
          ...prev,
          not_bros: true
        }));
        // Remove the optimistic message
        setConversationMessages(prev => prev.filter(m => !m.id?.startsWith('temp_')));
      }
    } catch (err) {
      console.error('Failed to send message:', err);
    }
  };
  
  // Handle media upload - opens modal for normal/private choice
  const handleMediaUpload = (e) => {
    const file = e.target.files?.[0];
    if (!file || !activeConversation) return;

    const isImage = file.type.startsWith('image/');
    const isVideo = file.type.startsWith('video/');
    if (isImage && file.size > 10 * 1024 * 1024) {
      toast.error('L\'image ne doit pas dépasser 10 Mo');
      if (mediaInputRef.current) mediaInputRef.current.value = '';
      return;
    }
    if (isVideo && file.size > 50 * 1024 * 1024) {
      toast.error('La vidéo ne doit pas dépasser 50 Mo');
      if (mediaInputRef.current) mediaInputRef.current.value = '';
      return;
    }

    const previewUrl = isImage ? URL.createObjectURL(file) : null;
    setPendingMedia({ file, isImage, isVideo, previewUrl });
    setShowMediaModal(true);
    if (mediaInputRef.current) mediaInputRef.current.value = '';
  };

  // Send media (called from modal)
  const sendMedia = async (file, isPrivate = false) => {
    setShowMediaModal(false);
    const isImage = file.type.startsWith('image/');

    const token = localStorage.getItem('brozr_token');
    if (!token) return;

    const formData = new FormData();
    formData.append('file', file);
    if (activeConversation.id && !activeConversation.id.startsWith('temp_')) {
      formData.append('conversation_id', activeConversation.id);
    }
    if (activeConversation.is_group && activeConversation.participants) {
      formData.append('is_group', 'true');
      formData.append('participants', JSON.stringify(activeConversation.participants.map(p => p.credential_id || p.id)));
    } else if (activeConversation.partner_id) {
      formData.append('partner_id', activeConversation.partner_id);
    }
    if (isPrivate) formData.append('is_private', 'true');

    // Show optimistic preview
    const tempMsg = {
      id: `temp_media_${Date.now()}`,
      sender_id: user?.id,
      content: isImage ? '[Image]' : '[Média]',
      media_url: URL.createObjectURL(file),
      media_type: file.type,
      is_private: isPrivate,
      created_at: new Date().toISOString(),
      uploading: true
    };
    setConversationMessages(prev => [...prev, tempMsg]);

    try {
      const res = await fetch(`${API_URL}/api/messages/send-media`, {
        method: 'POST',
        headers: { 'Authorization': `Bearer ${token}` },
        body: formData
      });

      if (res.ok) {
        const json = await res.json();
        const data = json.data || json;
        if (activeConversation.id?.startsWith('temp_')) {
          setActiveConversation(prev => ({
            ...prev,
            id: data.conversation_id
          }));
        }
        fetchConversationMessages(data.conversation_id);
      }
    } catch (err) {
      console.error('Failed to upload media:', err);
    }

    // Clean up pending media preview URL
    if (pendingMedia?.previewUrl) {
      URL.revokeObjectURL(pendingMedia.previewUrl);
    }
    setPendingMedia(null);
  };

  // Handle report message/profile
  const handleReportClick = (type, id, userId, userName) => {
    setReportTarget({ type, id, user_id: userId, user_name: userName });
    setShowReportOverlay(true);
  };

  // Submit report
  const submitReport = async (reason) => {
    if (!reportTarget) return;
    
    try {
      const token = localStorage.getItem('brozr_token');
      if (!token) return;
      
      await fetch(`${API_URL}/api/interactions/report`, {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${token}`,
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({
          reported_user_id: reportTarget.user_id,
          reason: reason,
          category: reportTarget.type // 'message' or 'profile'
        })
      });
    } catch (err) {
      console.error('Failed to submit report:', err);
    }
    
    setShowReportOverlay(false);
    setReportTarget(null);
    setShowReportConfirm(true);
    setTimeout(() => setShowReportConfirm(false), 3000);
  };

  // Handle remove Bro
  const handleRemoveBro = async () => {
    if (!broToRemove) return;
    
    try {
      const token = localStorage.getItem('brozr_token');
      if (!token) return;
      
      const res = await fetch(`${API_URL}/api/follows/${broToRemove.id}`, {
        method: 'DELETE',
        headers: { 'Authorization': `Bearer ${token}` }
      });
      
      if (res.ok) {
        // Immediately remove from local state for instant UI feedback
        setFollowing(prev => prev.filter(f => f.id !== broToRemove.id));
        setShowRemoveBroConfirm(false);
        setBroToRemove(null);
        setSelectedFollower(null);
      } else {
        console.error('Failed to unfollow:', await res.text());
      }
    } catch (err) {
      console.error('Failed to remove bro:', err);
    }
  };
  
  // Fetch messages for active conversation
  const fetchConversationMessages = async (conversationId) => {
    if (!conversationId || conversationId.startsWith('temp_')) return;
    
    try {
      const token = localStorage.getItem('brozr_token');
      if (!token) return;
      
      const res = await fetch(`${API_URL}/api/messages/conversations/${conversationId}/messages?page=1&per_page=50`, {
        headers: { 'Authorization': `Bearer ${token}` }
      });
      if (res.ok) {
        const json = await res.json();
        const payload = json.data || json;
        // API returns paginated: { items: [...], total, page, ... }
        const msgs = Array.isArray(payload) ? payload : (payload.items || []);
        // Messages come newest-first from API, reverse for display (oldest at top)
        setConversationMessages([...msgs].reverse());
      }
    } catch (err) {
      console.error('Failed to fetch messages:', err);
    }
  };
  
  // Mark conversation as read on server
  const markConversationRead = async (conversationId) => {
    if (!conversationId || conversationId.startsWith('temp_')) return;
    try {
      const token = localStorage.getItem('brozr_token');
      if (!token) return;
      await fetch(`${API_URL}/api/messages/conversations/${conversationId}/read`, {
        method: 'POST',
        headers: { 'Authorization': `Bearer ${token}` }
      });
    } catch (err) {
      console.error('Failed to mark as read:', err);
    }
  };

  // Load messages when conversation changes + mark as read
  useEffect(() => {
    if (activeConversation && !activeConversation.id?.startsWith('temp_')) {
      fetchConversationMessages(activeConversation.id);
      markConversationRead(activeConversation.id);
      // Reset unread count locally
      setMessages(prev => prev.map(conv =>
        conv.id === activeConversation.id ? { ...conv, unread_count: 0 } : conv
      ));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeConversation?.id]);
  
  // Check Bros status whenever following list or active conversation changes
  useEffect(() => {
    if (activeConversation && !activeConversation.is_group && activeConversation.partner_id) {
      const isBro = following.some(f => f.id === activeConversation.partner_id || f.credential_id === activeConversation.partner_id);
      if (!isBro && !activeConversation.not_bros) {
        setActiveConversation(prev => prev ? { ...prev, not_bros: true } : null);
      } else if (isBro && activeConversation.not_bros) {
        // If somehow they became Bros again, remove the flag
        setActiveConversation(prev => prev ? { ...prev, not_bros: false } : null);
      }
    }
  }, [activeConversation?.partner_id, activeConversation?.is_group, activeConversation?.not_bros, following]);
  
  // Scroll to bottom when messages load
  useEffect(() => {
    if (conversationMessages.length > 0) {
      setTimeout(() => {
        messagesEndRef.current?.scrollIntoView({ behavior: 'auto' });
      }, 100);
    }
  }, [conversationMessages]);

  // Filter followers by search query
  const filteredFollowers = following.filter(f => 
    f.display_name?.toLowerCase().includes(followerSearchQuery.toLowerCase())
  );
  
  // Filter contacts for new message
  const filteredContacts = following.filter(f =>
    f.display_name?.toLowerCase().includes(contactSearchQuery.toLowerCase())
  );

  // Search bros by pseudo (debounced)
  useEffect(() => {
    if (addBroDebounceRef.current) clearTimeout(addBroDebounceRef.current);
    if (!addBroQuery.trim()) {
      setAddBroResults([]);
      setAddBroLoading(false);
      return;
    }
    setAddBroLoading(true);
    addBroDebounceRef.current = setTimeout(async () => {
      try {
        const token = localStorage.getItem('brozr_token');
        const res = await fetch(`${API_URL}/api/users/search?q=${encodeURIComponent(addBroQuery.trim())}&limit=30`, {
          headers: { 'Authorization': `Bearer ${token}` }
        });
        if (res.ok) {
          const json = await res.json();
          setAddBroResults(json.data || []);
        }
      } catch (err) {
        console.error('Search bros failed:', err);
      } finally {
        setAddBroLoading(false);
      }
    }, 300);
    return () => clearTimeout(addBroDebounceRef.current);
  }, [addBroQuery]);

  const handleAddBroFollow = async (targetId) => {
    try {
      const token = localStorage.getItem('brozr_token');
      const res = await fetch(`${API_URL}/api/users/follows/${targetId}`, {
        method: 'POST',
        headers: { 'Authorization': `Bearer ${token}` }
      });
      if (res.ok) {
        setPendingFollows(prev => new Set([...prev, targetId]));
      } else {
        const err = await res.json().catch(() => ({}));
        toast.error(err.error || 'Erreur lors de l\'envoi');
      }
    } catch (err) {
      toast.error('Erreur réseau');
    }
  };

  // Format message time with date
  const formatMessageTime = (timestamp) => {
    if (!timestamp) return '';
    
    // Handle ISO format dates
    let date;
    try {
      date = new Date(timestamp);
      if (isNaN(date.getTime())) return '';
    } catch (e) {
      return '';
    }
    
    const now = new Date();
    
    // Reset hours to compare days properly
    const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
    const messageDate = new Date(date.getFullYear(), date.getMonth(), date.getDate());
    const diffTime = today.getTime() - messageDate.getTime();
    const diffDays = Math.floor(diffTime / (1000 * 60 * 60 * 24));
    
    const timeStr = date.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' });
    
    if (diffDays === 0) {
      // Today: show "Auj." + time
      return `Auj. ${timeStr}`;
    } else if (diffDays === 1) {
      // Yesterday: show "Hier" + time
      return `Hier ${timeStr}`;
    } else if (diffDays < 7) {
      // Within a week: show weekday + time
      const weekday = date.toLocaleDateString('fr-FR', { weekday: 'short' });
      return `${weekday} ${timeStr}`;
    } else {
      // Older: show date + time
      const dateStr = date.toLocaleDateString('fr-FR', { day: '2-digit', month: '2-digit' });
      return `${dateStr} ${timeStr}`;
    }
  };

  return (
    <div className="fixed inset-0 bg-black flex items-center justify-center">
      {/* Mobile-width container for desktop */}
      <div className="relative w-full h-full max-w-[430px] mx-auto bg-black flex flex-col overflow-hidden">
        
        {/* Header - Fixed at top, SAME positioning as LivePrematch */}
        <div className="flex-shrink-0 flex items-center justify-between px-3 py-3 bg-black">
          {/* Left: Profile Photo */}
          <button 
            onClick={() => navigate('/profile')} 
            className="relative"
            data-testid="space-profile-btn"
          >
            <div className="w-9 h-9 rounded-full border-2 border-white overflow-hidden shadow-lg">
              {user?.profile_photo ? (
                <img src={user.profile_photo} alt="" className="w-full h-full object-cover" />
              ) : (
                <div className="w-full h-full bg-white/10 flex items-center justify-center">
                  <svg className="w-4 h-4 text-white/70" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
                  </svg>
                </div>
              )}
            </div>
          </button>

          {/* Right: Notification Bell + Account Menu */}
          <div className="flex items-center gap-2">
            <button 
              onClick={handleNotifClick} 
              className="relative w-9 h-9 flex items-center justify-center bg-white/10 rounded-full"
              data-testid="space-notif-btn"
            >
              <svg className="w-4 h-4 text-white" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                <path d="M12 2C10.3431 2 9 3.34315 9 5V5.34141C6.66962 6.16508 5 8.38756 5 11V14.1585C5 14.6973 4.78595 15.2141 4.40493 15.5951L3 17H21L19.5951 15.5951C19.2141 15.2141 19 14.6973 19 14.1585V11C19 8.38756 17.3304 6.16508 15 5.34141V5C15 3.34315 13.6569 2 12 2Z" strokeLinecap="round" strokeLinejoin="round"/>
                <path d="M9 17V18C9 19.6569 10.3431 21 12 21C13.6569 21 15 19.6569 15 18V17" strokeLinecap="round" strokeLinejoin="round"/>
              </svg>
              {unreadCount > 0 && (
                <div className="absolute -top-1 -right-1 min-w-[14px] h-[14px] bg-red-500 rounded-full flex items-center justify-center px-0.5">
                  <span className="text-white text-[8px] font-bold">{unreadCount > 9 ? '9+' : unreadCount}</span>
                </div>
              )}
            </button>

          </div>
        </div>

        {/* Notifications Panel */}
        {showNotif && (
          <div className="absolute top-14 right-4 w-72 bg-black/95 rounded-2xl border border-white/10 z-30 max-h-[60vh] flex flex-col">
            <div className="p-4 border-b border-white/10 flex justify-between flex-shrink-0">
              <span className="text-white font-bold">Notifications</span>
              <button onClick={() => setShowNotif(false)} className="text-white/50 hover:text-white">✕</button>
            </div>
            <div className="p-4 overflow-y-auto flex-1">
              {notifications.length === 0 ? (
                <p className="text-white/50 text-sm text-center">Aucune notification</p>
              ) : (
                <div className="space-y-3">
                  {notifications
                    .filter(notif => notif.type !== 'livecam_response') // Hide livecam response notifications
                    .map((notif) => (
                    <div key={notif.id} className={`p-3 rounded-lg ${notif.read ? 'bg-white/5' : 'bg-white/20 border border-white/30'}`}>
                      <p className="text-white/90 text-sm">
                        {notif.type === 'follow_request' && (
                          <>👋 <span className="font-bold">@{notif.from_user?.display_name || notif.from_user_name || 'Bro'}</span> veut te follow</>
                        )}
                        {notif.type === 'follow_accepted' && (
                          <>✅ <span className="font-bold">@{notif.from_user?.display_name || notif.from_user_name || 'Bro'}</span> a accepté ta demande</>
                        )}
                        {notif.type === 'like' && (
                          <>❤️ <span className="font-bold">@{notif.from_user?.display_name || notif.from_user_name || 'Bro'}</span> t&apos;a liké</>
                        )}
                        {notif.type === 'new_message' && (
                          <>💬 <span className="font-bold">@{notif.from_user?.display_name || notif.from_user_name || 'Bro'}</span> t&apos;a envoyé un message</>
                        )}
                        {notif.type === 'livecam_request' && (
                          <>📹 <span className="font-bold">@{notif.from_user?.display_name || notif.from_user_name || 'Bro'}</span> te demande un Live Cam</>
                        )}
                        {notif.type === 'welcome' && '🎉 Bienvenue sur Brozr!'}
                        {/* Fallback for unknown types - show message_preview if available */}
                        {!['follow_request', 'follow_accepted', 'like', 'new_message', 'livecam_request', 'livecam_response', 'welcome'].includes(notif.type) && (
                          notif.message_preview || <>🔔 Notification</>
                        )}
                      </p>
                      <p className="text-white/40 text-xs mt-1">
                        {new Date(notif.created_at).toLocaleDateString('fr-FR', { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' })}
                      </p>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        )}


        {/* Main Content - Scrollable */}
        <div className="flex-1 overflow-y-auto px-4 pt-4">
          
          {/* Section: Mes Bros. (users with accepted follow relationship) */}
          <div className="mb-6">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-white font-bold text-lg" style={{ fontFamily: 'Unbounded, sans-serif' }}>
                Mes Bros.
              </h2>
              <div className="flex items-center gap-3">
                <button
                  onClick={() => { setShowAddBro(true); setAddBroQuery(''); setAddBroResults([]); }}
                  className="w-7 h-7 flex items-center justify-center hover:opacity-80 transition-opacity"
                >
                  <img src="/add-bro.svg" alt="Ajouter un Bro" className="w-7 h-7" />
                </button>
                <button
                  className="flex items-center gap-1 text-white/60 hover:text-white transition-colors"
                  onClick={() => setShowAllFollowers(true)}
                >
                  <span className="text-sm">Voir tout</span>
                  <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <path strokeLinecap="round" strokeLinejoin="round" d="M9 5l7 7-7 7" />
                  </svg>
                </button>
              </div>
            </div>
            
            {/* Horizontal Scroll for Followers */}
            <div className="overflow-x-auto scrollbar-hide -mx-4 px-4">
              <div className="flex gap-3 pb-2">
                {loadingFollowing ? (
                  <div className="flex gap-3">
                    {[1,2,3,4,5].map((i) => (
                      <div key={i} className="flex flex-col items-center gap-1.5 animate-pulse">
                        <div className="w-14 h-14 rounded-full bg-white/10"></div>
                        <div className="w-10 h-2.5 rounded bg-white/10"></div>
                      </div>
                    ))}
                  </div>
                ) : following.length === 0 ? (
                  <div className="flex items-center justify-center w-full py-4">
                    <p className="text-white/40 text-sm">Aucun Bro pour le moment</p>
                  </div>
                ) : (
                  following.map((followedUser) => (
                    <button 
                      key={followedUser.id} 
                      onClick={() => handleFollowerClick(followedUser)}
                      className="flex flex-col items-center gap-1.5 flex-shrink-0"
                    >
                      <div className="relative">
                        <div className="w-14 h-14 rounded-full border-2 border-white overflow-hidden">
                          {followedUser.profile_photo ? (
                            <img src={followedUser.profile_photo} alt="" className="w-full h-full object-cover" />
                          ) : (
                            <div className="w-full h-full bg-white/10 flex items-center justify-center">
                              <svg className="w-5 h-5 text-white/70" viewBox="0 0 24 24" fill="currentColor">
                                <path d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"/>
                              </svg>
                            </div>
                          )}
                        </div>
                        {/* Online indicator (green dot) */}
                        {followedUser.is_online && (
                          <div className="absolute bottom-0 right-0 w-3.5 h-3.5 bg-green-500 border-2 border-black rounded-full"></div>
                        )}
                      </div>
                      <span className="text-white/80 text-[10px] font-medium truncate max-w-[56px]">
                        @{followedUser.display_name || 'User'}
                      </span>
                    </button>
                  ))
                )}
              </div>
            </div>
          </div>

          {/* Section: Mes Messages */}
          <div className="pb-4">
            <div className="flex items-center justify-between mb-3">
              <h2 className="text-white font-bold text-lg" style={{ fontFamily: 'Unbounded, sans-serif' }}>
                Mes messages
              </h2>
              <button 
                onClick={handleNewMessage}
                className="w-7 h-7 flex items-center justify-center bg-white rounded-full"
                data-testid="space-new-message-btn"
              >
                <svg className="w-4 h-4 text-black" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
                  <path strokeLinecap="round" strokeLinejoin="round" d="M12 4v16m8-8H4" />
                </svg>
              </button>
            </div>

            {/* Messages List */}
            <div className="space-y-2">
              {loadingMessages ? (
                <div className="space-y-2">
                  {[1,2,3].map((i) => (
                    <div key={i} className="flex items-center gap-3 p-3 rounded-xl bg-white/5 animate-pulse">
                      <div className="w-12 h-12 rounded-full bg-white/10"></div>
                      <div className="flex-1">
                        <div className="w-24 h-4 rounded bg-white/10 mb-2"></div>
                        <div className="w-40 h-3 rounded bg-white/10"></div>
                      </div>
                    </div>
                  ))}
                </div>
              ) : messages.length === 0 ? (
                <div className="flex flex-col items-center justify-center py-10">
                  <div className="w-14 h-14 rounded-full bg-white/5 flex items-center justify-center mb-3">
                    <svg className="w-7 h-7 text-white/30" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                      <path strokeLinecap="round" strokeLinejoin="round" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
                    </svg>
                  </div>
                  <p className="text-white/40 text-sm text-center">Aucun message pour le moment</p>
                  <p className="text-white/30 text-xs text-center mt-1">Commencez une conversation avec vos followers</p>
                </div>
              ) : (
                messages.map((conversation) => {
                  const hasUnread = conversation.unread_count > 0;
                  // Check if partner is still a Bro
                  const isBro = following.some(f => f.id === conversation.partner_id || f.credential_id === conversation.partner_id);
                  const partnerProfile = following.find(f => f.id === conversation.partner_id || f.credential_id === conversation.partner_id);
                  const partnerName = conversation.partner_name || partnerProfile?.display_name || 'Inconnu';
                  return (
                  <button 
                    key={conversation.id}
                    onClick={() => {
                      setActiveConversation({
                        ...conversation,
                        partner_name: partnerName,
                        not_bros: !isBro
                      });
                      setConversationMessages([]);
                    }}
                    className={`w-full flex items-center gap-3 p-3 rounded-xl transition-all ${
                      hasUnread 
                        ? 'bg-white/10 border border-white/20' 
                        : 'bg-white/5 hover:bg-white/10'
                    }`}
                  >
                    <div className="relative flex-shrink-0">
                      <div className={`w-12 h-12 rounded-full border-2 overflow-hidden ${hasUnread ? 'border-white' : 'border-white/50'}`}>
                        {conversation.is_group ? (
                          conversation.group_photo ? (
                            <img src={conversation.group_photo} alt="" className="w-full h-full object-cover" />
                          ) : (
                            <div className="w-full h-full bg-white/10 flex items-center justify-center">
                              <svg className="w-6 h-6 text-white" viewBox="0 0 24 24" fill="currentColor">
                                <path d="M12 6c1.1 0 2 .9 2 2s-.9 2-2 2-2-.9-2-2 .9-2 2-2m0 10c2.7 0 5.8 1.29 6 2H6c.23-.72 3.31-2 6-2m0-12C9.79 4 8 5.79 8 8s1.79 4 4 4 4-1.79 4-4-1.79-4-4-4zm0 10c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"/>
                              </svg>
                            </div>
                          )
                        ) : conversation.partner_photo ? (
                          <img src={conversation.partner_photo} alt="" className="w-full h-full object-cover" />
                        ) : (
                          <div className="w-full h-full bg-white/10 flex items-center justify-center">
                            <svg className="w-6 h-6 text-white/70" viewBox="0 0 24 24" fill="currentColor">
                              <path d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"/>
                            </svg>
                          </div>
                        )}
                      </div>
                      {/* Only show online status if they are still a Bro (not for groups) */}
                      {!conversation.is_group && isBro && conversation.partner_online && (
                        <div className="absolute bottom-0 right-0 w-3 h-3 bg-green-500 border-2 border-black rounded-full"></div>
                      )}
                      {hasUnread && (
                        <div className="absolute -top-1 -left-1 min-w-[18px] h-[18px] bg-red-500 rounded-full flex items-center justify-center px-1">
                          <span className="text-white text-[10px] font-bold">{conversation.unread_count}</span>
                        </div>
                      )}
                    </div>
                    <div className="flex-1 min-w-0 text-left">
                      <div className="flex items-center gap-2">
                        <span className={`font-bold text-sm ${hasUnread ? 'text-white' : 'text-white/80'}`}>
                          {conversation.is_group
                            ? (conversation.group_name || conversation.name || 'Groupe')
                            : `@${partnerName}`}
                        </span>
                        {!conversation.is_group && conversation.partner_country && (
                          <span className="text-xs">{conversation.partner_country}</span>
                        )}
                      </div>
                      <p className={`text-xs truncate mt-0.5 ${hasUnread ? 'text-white/80 font-medium' : 'text-white/50'}`}>{conversation.last_message || 'Aucun message'}</p>
                    </div>
                    <span className={`text-[10px] flex-shrink-0 ${hasUnread ? 'text-white/50' : 'text-white/30'}`}>
                      {formatMessageTime(conversation.last_message_time)}
                    </span>
                  </button>
                  );
                })
              )}
            </div>
          </div>
        </div>

        {/* ===== VIDEO CALL OVERLAYS ===== */}

        {/* Incoming Call Overlay */}
        {callState === 'incoming' && callData && (
          <div className="fixed inset-0 bg-black/95 z-[9999] flex items-center justify-center animate-fade-in">
            <div className="w-full max-w-[350px] mx-4 bg-gradient-to-b from-[#1a1a1a] to-[#0d0d0d] rounded-3xl p-6 text-center border border-white/10">
              {/* Caller Photo */}
              <div className="relative mx-auto w-24 h-24 mb-4">
                <div className="w-24 h-24 rounded-full border-4 border-white/30 overflow-hidden animate-pulse-ring">
                  {callData.caller_photo ? (
                    <img src={callData.caller_photo} alt="" className="w-full h-full object-cover" />
                  ) : (
                    <div className="w-full h-full bg-white/10 flex items-center justify-center">
                      <span className="text-white text-3xl font-bold">
                        {(callData.caller_name || '?')[0]?.toUpperCase()}
                      </span>
                    </div>
                  )}
                </div>
                {/* Video icon badge */}
                <div className="absolute -bottom-1 -right-1 w-8 h-8 bg-white rounded-full flex items-center justify-center">
                  <svg className="w-4 h-4 text-black" fill="currentColor" viewBox="0 0 24 24">
                    <path d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
                  </svg>
                </div>
              </div>

              {/* Caller Info */}
              <h3 className="text-white text-xl font-bold mb-1">@{callData.caller_name}</h3>
              <p className="text-white/50 text-sm mb-6">Appel vidéo entrant</p>

              {/* Action Buttons */}
              <div className="flex items-center justify-center gap-6">
                {/* Accept - GAUCHE, VERT */}
                <button
                  onClick={acceptCall}
                  className="w-14 h-14 bg-green-500 hover:bg-green-600 rounded-full flex items-center justify-center transition-all"
                >
                  <svg className="w-7 h-7 text-white" fill="currentColor" viewBox="0 0 24 24">
                    <path d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
                  </svg>
                </button>

                {/* Decline - MILIEU, ROUGE */}
                <button
                  onClick={declineCall}
                  className="w-14 h-14 bg-red-500 hover:bg-red-600 rounded-full flex items-center justify-center transition-all"
                >
                  <svg className="w-7 h-7 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2">
                    <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>

                {/* Ignore - DROITE */}
                <button
                  onClick={handleIgnoreCall}
                  className="w-14 h-14 bg-white/10 hover:bg-white/20 rounded-full flex flex-col items-center justify-center transition-all"
                >
                  <svg className="w-5 h-5 text-white/70" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2">
                    <path strokeLinecap="round" strokeLinejoin="round" d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636" />
                  </svg>
                  <span className="text-[8px] text-white/40 mt-0.5">60 min</span>
                </button>
              </div>

              {/* Labels */}
              <div className="flex items-center justify-center gap-6 mt-3">
                <span className="w-14 text-center text-white text-xs font-medium">Accepter</span>
                <span className="w-14 text-center text-white/40 text-xs">Refuser</span>
                <span className="w-14 text-center text-white/30 text-xs">Ignorer</span>
              </div>
            </div>
          </div>
        )}

        {/* Outgoing Call Overlay */}
        {callState === 'outgoing' && callData && (
          <div className="fixed inset-0 bg-black/95 z-[9999] flex items-center justify-center animate-fade-in">
            <div className="w-full max-w-[350px] mx-4 bg-gradient-to-b from-[#1a1a1a] to-[#0d0d0d] rounded-3xl p-6 text-center border border-white/10">
              {/* Partner Photo */}
              <div className="relative mx-auto w-24 h-24 mb-4">
                <div className="w-24 h-24 rounded-full border-4 border-white/30 overflow-hidden animate-pulse">
                  {callData.partner_photo ? (
                    <img src={callData.partner_photo} alt="" className="w-full h-full object-cover" />
                  ) : (
                    <div className="w-full h-full bg-white/10 flex items-center justify-center">
                      <span className="text-white text-3xl font-bold">
                        {(callData.partner_name || '?')[0]?.toUpperCase()}
                      </span>
                    </div>
                  )}
                </div>
                {/* Video icon badge */}
                <div className="absolute -bottom-1 -right-1 w-8 h-8 bg-white/20 rounded-full flex items-center justify-center">
                  <svg className="w-4 h-4 text-white/60" fill="currentColor" viewBox="0 0 24 24">
                    <path d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
                  </svg>
                </div>
              </div>

              {/* Partner Info */}
              <h3 className="text-white text-xl font-bold mb-1">@{callData.partner_name}</h3>
              <p className="text-white/50 text-sm mb-6">Appel en cours...</p>

              {/* Cancel Button */}
              <button
                onClick={endCall}
                className="w-14 h-14 mx-auto bg-red-500/20 hover:bg-red-500/80 rounded-full flex items-center justify-center transition-all"
              >
                <svg className="w-7 h-7 text-red-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2">
                  <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
              <span className="block text-center text-red-400/60 text-xs mt-2">Annuler</span>
            </div>
          </div>
        )}

        {/* Video elements — always in DOM, hidden when not active (prevents React unmount killing streams) */}
        <video
          ref={remoteVideoRef}
          autoPlay
          playsInline
          className={callState === 'active' ? "fixed inset-0 w-full h-full object-cover z-[9999] pointer-events-none" : "hidden"}
        />
        <video
          ref={localVideoRef}
          autoPlay
          playsInline
          muted
          className={callState === 'active' ? "fixed top-3 right-3 w-24 h-32 object-cover rounded-2xl border-2 border-white/30 shadow-xl z-[10002] pointer-events-none" : "hidden"}
          style={callState === 'active' ? { transform: 'scaleX(-1)' } : undefined}
        />

        {/* Active Call View — identical to VideoCall.js connected UI */}
        {callState === 'active' && (() => {
          const p = callPartnerProfile;
          const partnerName = p?.display_name || callData?.partner_name || callData?.caller_name || 'Bro';
          const partnerPhoto = p?.profile_photo || callData?.partner_photo || callData?.caller_photo;
          const partnerAge = p?.age;
          const partnerKinks = p?.kinks || [];
          const flagMap = { FR: '\u{1F1EB}\u{1F1F7}', BE: '\u{1F1E7}\u{1F1EA}', CH: '\u{1F1E8}\u{1F1ED}', CA: '\u{1F1E8}\u{1F1E6}', US: '\u{1F1FA}\u{1F1F8}', UK: '\u{1F1EC}\u{1F1E7}', DE: '\u{1F1E9}\u{1F1EA}', ES: '\u{1F1EA}\u{1F1F8}', IT: '\u{1F1EE}\u{1F1F9}', NL: '\u{1F1F3}\u{1F1F1}', PT: '\u{1F1F5}\u{1F1F9}', AT: '\u{1F1E6}\u{1F1F9}' };
          const partnerFlag = p?.country ? (flagMap[p.country] || '') : '';

          // Kinks rendering — matching kinks first (like VideoCall)
          const renderCallKinks = () => {
            if (partnerKinks.length === 0) return null;
            let matchingKinks = [];
            try {
              const saved = localStorage.getItem('brozr_match_filters');
              if (saved) { matchingKinks = JSON.parse(saved).kinks || []; }
            } catch (e) {}
            // Fallback: si pas de filtre kinks, comparer avec les kinks du profil user
            if (matchingKinks.length === 0) {
              matchingKinks = user?.kinks || [];
              if (matchingKinks.length === 0) {
                try { matchingKinks = JSON.parse(localStorage.getItem('brozr_user') || '{}').kinks || []; } catch (e) {}
              }
            }
            const sorted = [];
            const others = [];
            for (let i = 0; i < partnerKinks.length; i++) {
              if (matchingKinks.includes(partnerKinks[i])) sorted.push(partnerKinks[i]);
              else others.push(partnerKinks[i]);
            }
            const all = sorted.concat(others);
            const show = all.slice(0, 3);
            const rest = all.length - 3;
            const els = [];
            for (let i = 0; i < show.length; i++) {
              const isMatch = matchingKinks.includes(show[i]);
              const cls = matchingKinks.length === 0
                ? 'px-2 py-0.5 rounded-full text-[10px] whitespace-nowrap bg-white/10 text-white border border-white/20'
                : isMatch
                ? 'px-2 py-0.5 rounded-full text-[10px] whitespace-nowrap bg-white text-black font-semibold'
                : 'px-2 py-0.5 rounded-full text-[10px] whitespace-nowrap bg-white/5 text-white/50';
              els.push(<span key={`ck-${i}`} className={cls}>{show[i]}</span>);
            }
            if (rest > 0) els.push(<button key="ck-more" onClick={() => setShowCallKinksOverlay(true)} className="text-white font-bold text-[10px] ml-1">+{rest}</button>);
            return els;
          };

          return (
          <>
          {/* Black background behind video */}
          <div className="fixed inset-0 bg-black z-[9998]" />
          {/* Controls overlay */}
          <div className="fixed inset-0 z-[10001]">

            {/* TOP LEFT - Close (end call) */}
            <div className="absolute top-3 left-3 z-10">
              <div className="flex items-center gap-1.5 px-1.5 py-1.5 rounded-full bg-black/60 backdrop-blur-md">
                <button onClick={endCall} className="w-7 h-7 rounded-full bg-[#3a3a3a] flex items-center justify-center text-white hover:bg-[#4a4a4a] transition-all">
                  <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2.5">
                    <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </div>
            </div>

            {/* Timer pill - top center */}
            <div className="absolute top-4 left-1/2 -translate-x-1/2 z-10">
              <div className="bg-black/50 px-3 py-1 rounded-full">
                <span className="text-white text-sm font-mono">
                  {String(Math.floor(callDuration / 60)).padStart(2, '0')}:{String(callDuration % 60).padStart(2, '0')}
                </span>
              </div>
            </div>

            {/* RIGHT SIDE - Gift & Like (same position as VideoCall) */}
            <div className="absolute right-3 top-44 z-10 flex flex-col gap-4 items-center">
              {/* Gift Button */}
              <button
                onClick={() => setShowCallGiftPopup(true)}
                className="w-11 h-11 rounded-full flex items-center justify-center hover:scale-110 transition-all bg-[#1a1a1a] border-2 border-white"
              >
                <svg className="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="white" strokeWidth="1.5">
                  <path d="M20 12v10H4V12M2 7h20v5H2zM12 22V7M12 7H7.5a2.5 2.5 0 110-5C11 2 12 7 12 7zM12 7h4.5a2.5 2.5 0 100-5C13 2 12 7 12 7z"/>
                </svg>
              </button>

              {/* Like Button */}
              <div className="relative flex flex-col items-center">
                <button
                  onClick={handleCallLike}
                  disabled={callPartnerLiked}
                  className="transition-all hover:scale-110 active:scale-95"
                >
                  <svg className="w-8 h-8" viewBox="0 0 24 24" fill={callPartnerLiked ? '#ffffff' : '#ff0033'}>
                    <path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z"/>
                  </svg>
                </button>
                <span key={`clc-${callLikesReceived}`} className="text-white text-[10px] font-medium mt-0.5">
                  {callLikesReceived >= 1000 ? `${(callLikesReceived / 1000).toFixed(1)}k` : callLikesReceived}
                </span>
              </div>
            </div>

            {/* NEXT BUTTON - same position as VideoCall */}
            <div className="absolute bottom-[85px] right-3 z-10 flex flex-col items-center gap-1">
              <button
                onClick={() => { endCall(); navigate('/video-call'); }}
                className="w-14 h-14 rounded-full bg-white flex items-center justify-center shadow-xl hover:bg-gray-100 active:scale-95 transition-all"
              >
                <svg className="w-6 h-6 text-black" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2.5">
                  <path strokeLinecap="round" strokeLinejoin="round" d="M13 7l5 5m0 0l-5 5m5-5H6" />
                </svg>
              </button>
              <span className="text-white text-xs font-medium drop-shadow-lg">Next</span>
            </div>

            {/* BOTTOM AREA - Partner Info identical to VideoCall */}
            <div className="absolute bottom-16 left-3 right-3 z-10">
              {/* Main row: Photo + Info + Follow inline */}
              <div className="flex items-start gap-2">
                {/* Profile Photo */}
                <div className="w-10 h-10 rounded-full border-2 border-white overflow-hidden flex-shrink-0 bg-[#2a2a2a]">
                  {partnerPhoto ? (
                    <img src={partnerPhoto} alt="" className="w-full h-full object-cover" />
                  ) : (
                    <div className="w-full h-full flex items-center justify-center">
                      <svg className="w-5 h-5 text-white/50" viewBox="0 0 24 24" fill="currentColor">
                        <circle cx="12" cy="8" r="4" />
                        <path d="M12 14c-6 0-8 3-8 6v1h16v-1c0-3-2-6-8-6z" />
                      </svg>
                    </div>
                  )}
                </div>

                {/* Info block */}
                <div className="flex flex-col gap-0.5 min-w-0">
                  {/* Row 1: Name + Age + Follow */}
                  <div className="flex items-center gap-1.5">
                    <span className="text-white font-bold text-sm drop-shadow-lg max-w-[70px] truncate">{partnerName}</span>
                    <span className="text-white/80 text-sm drop-shadow-lg">{partnerAge ? `${partnerAge} yo` : ''}</span>
                    <button
                      onClick={handleCallFollow}
                      disabled={callFollowStatus === 'pending'}
                      className={`px-3 py-1 rounded-full font-bold text-xs transition-all ml-2 ${
                        callFollowStatus === 'accepted'
                          ? 'bg-[#1a1a1a] text-white border border-white/20'
                          : callFollowStatus === 'pending'
                          ? 'bg-white/30 text-white/70'
                          : 'bg-white text-black hover:bg-gray-100 active:scale-95'
                      }`}
                    >
                      {callFollowStatus === 'accepted' ? '✓ Suivi' : callFollowStatus === 'pending' ? '...' : '+ Suivre'}
                    </button>
                  </div>
                  {/* Row 2: Country flag + distance */}
                  <div className="flex items-center gap-1.5 text-white/70 text-xs drop-shadow-lg">
                    {partnerFlag && <span>{partnerFlag}</span>}
                    {p?.distance != null && <span>{Math.round(p.distance)} km</span>}
                  </div>
                </div>
              </div>

              {/* Kinks Row - aligned left */}
              <div className="flex items-center gap-1 mt-2 ml-0">
                {renderCallKinks()}
              </div>
            </div>

            {/* BOTTOM FOOTER BAR - identical to VideoCall */}
            <div className="absolute bottom-0 left-0 right-0 z-10">
              {/* Chat input above footer (if open) */}
              {showCallChat && (
                <div className="px-4 pb-2">
                  <input
                    value={callChatText}
                    onChange={(e) => setCallChatText(e.target.value)}
                    placeholder="Envoie un message..."
                    className="w-full bg-black/80 border border-white/20 rounded-full px-4 py-2.5 text-white text-sm placeholder:text-white/30 focus:outline-none focus:border-white/40"
                    style={{ fontSize: '16px' }}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter' && callChatText.trim()) {
                        const partnerId = callData?.partner_id || callData?.caller_id;
                        const token = localStorage.getItem('brozr_token');
                        if (partnerId && token) {
                          fetch(`${API_URL}/api/messages/send`, {
                            method: 'POST',
                            headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
                            body: JSON.stringify({ partner_id: partnerId, content: callChatText.trim() }),
                          }).catch(() => {});
                        }
                        setCallChatText('');
                      }
                    }}
                  />
                </div>
              )}
              <div className="bg-black rounded-t-[16px] px-4 py-2">
                <div className="flex items-center justify-center gap-8">
                  {/* Chat */}
                  <button
                    onClick={() => setShowCallChat(c => !c)}
                    className={`w-9 h-9 rounded-full flex items-center justify-center transition-all ${showCallChat ? 'bg-white text-black' : 'bg-[#333] text-white hover:bg-[#444]'}`}
                  >
                    <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                      <path d="M21 15a2 2 0 01-2 2H7l-4 4V5a2 2 0 012-2h14a2 2 0 012 2v10z" strokeLinecap="round" strokeLinejoin="round"/>
                    </svg>
                  </button>
                  {/* Mic */}
                  <button
                    onClick={() => {
                      if (localStream) {
                        localStream.getAudioTracks().forEach(t => { t.enabled = !t.enabled; });
                        setIsMuted(m => !m);
                      }
                    }}
                    className={`w-9 h-9 rounded-full flex items-center justify-center transition-all ${isMuted ? 'bg-red-500 text-white' : 'bg-[#333] text-white hover:bg-[#444]'}`}
                  >
                    <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="1.5">
                      {isMuted ? (
                        <>
                          <path strokeLinecap="round" strokeLinejoin="round" d="M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z" />
                          <path strokeLinecap="round" strokeLinejoin="round" d="M17 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2" />
                        </>
                      ) : (
                        <path strokeLinecap="round" strokeLinejoin="round" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z" />
                      )}
                    </svg>
                  </button>
                  {/* Camera */}
                  <button
                    onClick={() => {
                      if (localStream) {
                        localStream.getVideoTracks().forEach(t => { t.enabled = !t.enabled; });
                        setIsCameraOff(c => !c);
                      }
                    }}
                    className={`w-9 h-9 rounded-full flex items-center justify-center transition-all ${isCameraOff ? 'bg-red-500 text-white' : 'bg-[#333] text-white hover:bg-[#444]'}`}
                  >
                    <svg className="w-4 h-4" viewBox="0 0 24 24" fill={isCameraOff ? 'none' : 'currentColor'} stroke={isCameraOff ? 'currentColor' : 'none'} strokeWidth="1.5">
                      {isCameraOff ? (
                        <path strokeLinecap="round" strokeLinejoin="round" d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636" />
                      ) : (
                        <path d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
                      )}
                    </svg>
                  </button>
                </div>
              </div>
            </div>
          </div>

          {/* Kinks overlay */}
          {showCallKinksOverlay && (() => {
            const partnerKinksAll = p?.kinks || [];
            let mkinks = [];
            try { const sv = localStorage.getItem('brozr_match_filters'); if (sv) { mkinks = JSON.parse(sv).kinks || []; } } catch (e) {}
            if (mkinks.length === 0) { mkinks = user?.kinks || []; if (mkinks.length === 0) { try { mkinks = JSON.parse(localStorage.getItem('brozr_user') || '{}').kinks || []; } catch (e) {} } }
            const sorted = []; const others = [];
            for (let i = 0; i < partnerKinksAll.length; i++) { if (mkinks.includes(partnerKinksAll[i])) sorted.push(partnerKinksAll[i]); else others.push(partnerKinksAll[i]); }
            const allK = sorted.concat(others);
            return (
              <div className="fixed inset-0 bg-black/80 z-[10003] flex items-end justify-center" onClick={() => setShowCallKinksOverlay(false)}>
                <div className="w-full max-w-md bg-[#1a1a1a] rounded-t-3xl border-t border-white/10 max-h-[60vh] flex flex-col" onClick={e => e.stopPropagation()}>
                  <div className="p-4 border-b border-white/10 flex justify-between items-center flex-shrink-0">
                    <h3 className="text-white font-bold text-lg">Kinks de {partnerName}</h3>
                    <button onClick={() => setShowCallKinksOverlay(false)} className="w-8 h-8 flex items-center justify-center text-white/50 hover:text-white text-xl">✕</button>
                  </div>
                  <div className="p-4 overflow-y-auto flex-1">
                    <div className="flex flex-wrap gap-2">
                      {allK.map((kink, i) => {
                        const isM = mkinks.includes(kink);
                        const cls = mkinks.length === 0
                          ? 'px-3 py-1.5 rounded-full text-sm bg-white/10 text-white'
                          : isM
                          ? 'px-3 py-1.5 rounded-full text-sm bg-white text-black font-medium'
                          : 'px-3 py-1.5 rounded-full text-sm bg-white/5 text-white/50';
                        return <span key={`ok-${i}`} className={cls}>{kink}</span>;
                      })}
                    </div>
                  </div>
                </div>
              </div>
            );
          })()}

          {/* Gift popup */}
          {showCallGiftPopup && (
            <div className="fixed inset-0 bg-black/60 z-[10002] flex items-center justify-center" onClick={() => setShowCallGiftPopup(false)}>
              <div className="bg-[#1a1a1a] border border-white/10 rounded-2xl p-6 mx-4 max-w-[280px] text-center" onClick={e => e.stopPropagation()}>
                <span className="text-4xl block mb-3">🎁</span>
                <h3 className="text-white font-bold text-lg mb-2">Cadeaux</h3>
                <p className="text-white/50 text-sm">Bient&ocirc;t disponible !</p>
                <button
                  onClick={() => setShowCallGiftPopup(false)}
                  className="mt-4 w-full py-2 bg-white text-black rounded-xl font-bold text-sm"
                >
                  OK
                </button>
              </div>
            </div>
          )}
          </>
          );
        })()}

        {/* ===== REPORT OVERLAY ===== */}
        {showReportOverlay && (
          <div className="fixed inset-0 bg-black/95 z-[9999] flex items-center sm:items-center justify-center">
            <div className="w-full max-w-[430px] mx-4 bg-gradient-to-b from-[#1a1a1a] to-[#0d0d0d] rounded-2xl border border-white/10 max-h-[60vh] flex flex-col">
              <div className="p-4 border-b border-white/10 flex justify-between items-center flex-shrink-0">
                <h3 className="text-white font-bold text-lg">
                  {reportStep === 1 ? `Signaler ${reportTarget?.user_name ? `@${reportTarget.user_name}` : ''}` : 'Ajouter un commentaire'}
                </h3>
                <button onClick={() => { setShowReportOverlay(false); setReportTarget(null); setReportStep(1); setReportReason(''); setReportComment(''); }} className="w-8 h-8 flex items-center justify-center text-white/50 hover:text-white text-xl">✕</button>
              </div>
              <div className="p-4 overflow-y-auto flex-1">
                {reportStep === 1 ? (
                  <div className="flex flex-col gap-2">
                    {['Activité illégale ou mineur', 'Harcèlement ou menace', 'Spam ou arnaque', 'Partage sans consentement', 'Autre'].map((reason) => (
                      <button
                        key={reason}
                        onClick={() => { setReportReason(reason); setReportStep(2); }}
                        className="w-full py-3 bg-white/10 hover:bg-white/20 text-white rounded-xl text-sm text-left px-4 transition-all"
                      >
                        {reason}
                      </button>
                    ))}
                  </div>
                ) : (
                  <div className="space-y-4">
                    <p className="text-white/60 text-sm">Raison : <span className="text-white font-medium">{reportReason}</span></p>
                    <textarea
                      value={reportComment}
                      onChange={(e) => setReportComment(e.target.value.slice(0, 150))}
                      placeholder="Ajoute un commentaire (optionnel)..."
                      maxLength={150}
                      className="w-full bg-white/5 border border-white/10 focus:border-white/30 focus:outline-none p-3 rounded-xl text-white min-h-[80px] resize-none placeholder:text-white/30 text-sm"
                      style={{ fontSize: '16px' }}
                    />
                    <p className="text-white/30 text-xs text-right">{reportComment.length}/150</p>
                    <button
                      onClick={() => {
                        submitReport(reportReason);
                        setReportStep(1);
                        setReportReason('');
                        setReportComment('');
                      }}
                      className="w-full py-3 bg-white text-black rounded-xl font-bold text-sm transition-all active:scale-[0.98]"
                    >
                      Envoyer le signalement
                    </button>
                  </div>
                )}
              </div>
            </div>
          </div>
        )}

        {/* ===== REPORT CONFIRMATION ===== */}
        {showReportConfirm && (
          <div className="fixed inset-0 bg-black/80 z-[9999] flex items-center justify-center">
            <div className="bg-gradient-to-b from-[#1a1a1a] to-[#0d0d0d] border border-white/10 rounded-2xl px-8 py-6 text-center mx-4 max-w-[320px]">
              <div className="w-14 h-14 mx-auto mb-4 rounded-full bg-white/10 flex items-center justify-center">
                <svg className="w-7 h-7 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                </svg>
              </div>
              <h3 className="text-white font-bold text-lg mb-2">Signalement envoyé</h3>
              <p className="text-white/50 text-sm">Merci de contribuer à la sécurité de Brozr.</p>
            </div>
          </div>
        )}

        {/* ===== REMOVE BRO CONFIRMATION ===== */}
        {showRemoveBroConfirm && broToRemove && (
          <div className="fixed inset-0 bg-black/95 z-[9999] flex items-center justify-center">
            <div className="bg-gradient-to-b from-[#1a1a1a] to-[#0d0d0d] border border-white/10 rounded-2xl p-6 mx-4 max-w-[320px] w-full">
              <div className="text-center mb-6">
                <div className="w-16 h-16 mx-auto mb-4 rounded-full border-2 border-white/20 overflow-hidden">
                  {broToRemove.profile_photo ? (
                    <img src={broToRemove.profile_photo} alt="" className="w-full h-full object-cover" />
                  ) : (
                    <div className="w-full h-full bg-white/10 flex items-center justify-center">
                      <svg className="w-8 h-8 text-white/70" viewBox="0 0 24 24" fill="currentColor">
                        <path d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"/>
                      </svg>
                    </div>
                  )}
                </div>
                <h3 className="text-white font-bold text-lg mb-2">Retirer @{broToRemove.display_name} ?</h3>
                <p className="text-white/50 text-sm">Tu ne verras plus ce Bro dans ta liste.</p>
              </div>
              <div className="flex gap-3">
                <button
                  onClick={() => {
                    setShowRemoveBroConfirm(false);
                    setBroToRemove(null);
                  }}
                  className="flex-1 py-3 bg-white/10 hover:bg-white/20 text-white rounded-full font-medium transition-colors"
                >
                  Annuler
                </button>
                <button
                  onClick={handleRemoveBro}
                  className="flex-1 py-3 bg-white text-black rounded-full font-bold hover:bg-white/90 transition-colors"
                >
                  Retirer
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Bottom Nav - Fixed, SAME ORDER as LivePrematch: Cam Live, Space, Play Show */}
        <nav className="flex-shrink-0 bg-black px-4 py-3 pb-6">
          <div className="flex justify-around max-w-md mx-auto">
            <button onClick={() => navigate('/live-prematch')} className="flex flex-col items-center gap-1 text-white/50">
              <svg className="w-6 h-6" fill="currentColor" viewBox="0 0 24 24"><path d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" /></svg>
              <span className="text-xs font-medium">Cam Live</span>
            </button>
            <button className="flex flex-col items-center gap-1 text-white relative">
              {(() => {
                const totalUnread = messages.reduce((sum, c) => sum + (c.unread_count || 0), 0);
                return totalUnread > 0 ? (
                  <div className="absolute -top-1 -right-1 min-w-[14px] h-[14px] bg-red-500 rounded-full flex items-center justify-center px-0.5 z-10">
                    <span className="text-white text-[8px] font-bold">{totalUnread > 9 ? '9+' : totalUnread}</span>
                  </div>
                ) : null;
              })()}
              <svg className="w-6 h-6" fill="currentColor" viewBox="0 0 24 24">
                <path d="M21 6h-2v9H6v2c0 .55.45 1 1 1h11l4 4V7c0-.55-.45-1-1-1zm-4 6V3c0-.55-.45-1-1-1H3c-.55 0-1 .45-1 1v14l4-4h10c.55 0 1-.45 1-1z"/>
              </svg>
              <span className="text-xs font-medium">Space</span>
            </button>
            <button onClick={() => navigate('/play-show')} className="flex flex-col items-center gap-1 text-white/50">
              <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <rect x="3" y="3" width="18" height="18" rx="3" strokeWidth={1.5} />
                <path d="M10 8l6 4-6 4V8z" strokeWidth={1.5} strokeLinejoin="round" />
              </svg>
              <span className="text-xs font-medium">Play Show</span>
            </button>
          </div>
        </nav>

        {/* ===== FOLLOWER PROFILE MODAL ===== */}
        {selectedFollower && (
          <div 
            className="absolute inset-0 bg-black/80 z-50 flex items-end justify-center"
            onClick={(e) => { if (e.target === e.currentTarget) setSelectedFollower(null); }}
          >
            <div className="relative w-full max-w-[430px] bg-gradient-to-b from-[#1a1a1a] to-[#0d0d0d] rounded-t-3xl overflow-hidden max-h-[85vh] flex flex-col animate-slide-up">
              {/* Top Bar - Report left, Close right */}
              <div className="absolute top-3 left-3 right-3 flex justify-between items-center z-10">
                {/* Report Button - Left side (shield with !) */}
                <button
                  onClick={() => handleReportClick('profile', selectedFollower.id, selectedFollower.id, selectedFollower.display_name)}
                  className="w-8 h-8 flex items-center justify-center bg-white/10 rounded-full"
                >
                  <svg className="w-4 h-4 text-white/70" viewBox="0 0 24 24" fill="currentColor">
                    <path d="M12 2L4 5v6.09c0 5.05 3.41 9.76 8 10.91 4.59-1.15 8-5.86 8-10.91V5l-8-3zm0 15c-.55 0-1-.45-1-1v-1c0-.55.45-1 1-1s1 .45 1 1v1c0 .55-.45 1-1 1zm1-4h-2V8h2v5z"/>
                  </svg>
                </button>
                
                {/* Close Button - Right side */}
                <button 
                  onClick={() => setSelectedFollower(null)}
                  className="w-8 h-8 flex items-center justify-center bg-white/10 rounded-full"
                >
                  <svg className="w-5 h-5 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2">
                    <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </div>
              
              {/* Drag Handle */}
              <div className="flex justify-center pt-3 pb-2">
                <div className="w-10 h-1 bg-white/30 rounded-full"></div>
              </div>
              
              {/* Profile Photo */}
              <div className="relative mx-auto mt-4">
                <div className="w-24 h-24 rounded-full border-3 border-white overflow-hidden">
                  {selectedFollower.profile_photo ? (
                    <img src={selectedFollower.profile_photo} alt="" className="w-full h-full object-cover" />
                  ) : (
                    <div className="w-full h-full bg-white/10 flex items-center justify-center">
                      <svg className="w-16 h-16 text-white/70" viewBox="0 0 24 24" fill="currentColor">
                        <path d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"/>
                      </svg>
                    </div>
                  )}
                </div>
                {/* Online indicator */}
                {selectedFollower.is_online && (
                  <div className="absolute bottom-1 right-1 w-5 h-5 bg-green-500 border-3 border-black rounded-full"></div>
                )}
              </div>
              
              {/* User Info */}
              <div className="text-center px-6 pt-3 pb-4">
                <div className="flex items-center justify-center gap-2">
                  <h3 className="text-white text-xl font-bold">{selectedFollower.display_name}</h3>
                </div>
                
                {/* Location & Age */}
                <div className="flex items-center justify-center gap-3 mt-2 text-white/60 text-sm">
                  {selectedFollower.country && (
                    <span className="flex items-center gap-1">
                      <span>🇫🇷</span>
                      <span>{selectedFollower.country}</span>
                    </span>
                  )}
                  {selectedFollower.age && (
                    <span>{selectedFollower.age} ans</span>
                  )}
                  {selectedFollower.distance && (
                    <span>{selectedFollower.distance} km</span>
                  )}
                </div>
                
                {/* Kinks - Simple display without highlighting */}
                {selectedFollower.kinks && selectedFollower.kinks.length > 0 && (
                  <div className="mt-3">
                    <div className="flex flex-wrap justify-center gap-1.5">
                      {selectedFollower.kinks.map((kink, index) => (
                        <span key={index} className="px-2.5 py-1 bg-white/10 text-white/80 text-xs rounded-full">
                          {kink}
                        </span>
                      ))}
                    </div>
                  </div>
                )}
              </div>
              
              {/* Action Buttons - Live Cam (white bg), Message, Suivi */}
              <div className="flex items-center justify-center gap-3 px-6 py-4">
                {/* Video Call Button - First, white background */}
                <button
                  onClick={() => startCall(selectedFollower.id, selectedFollower.display_name, selectedFollower.profile_photo, selectedFollower.is_online)}
                  className={`w-12 h-12 flex items-center justify-center rounded-full flex-shrink-0 ${selectedFollower.is_online === false ? 'bg-white/30 cursor-not-allowed' : 'bg-white'}`}
                  data-testid="profile-call-btn"
                >
                  <svg className="w-5 h-5 text-black" fill="currentColor" viewBox="0 0 24 24">
                    <path d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
                  </svg>
                </button>
                
                {/* Message Button - Second */}
                <button
                  onClick={() => handleStartConversation(selectedFollower)}
                  className="flex-1 flex items-center justify-center gap-2 py-3 bg-white text-black rounded-full font-bold"
                  data-testid="profile-message-btn"
                >
                  <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <path strokeLinecap="round" strokeLinejoin="round" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
                  </svg>
                  <span className="text-sm">Message</span>
                </button>
                
                {/* Suivi Button - Last, smaller width, centered text */}
                <button
                  onClick={() => {
                    setBroToRemove(selectedFollower);
                    setShowRemoveBroConfirm(true);
                  }}
                  className="w-24 flex items-center justify-center gap-1.5 py-3 bg-black border border-white/30 text-white rounded-full font-medium"
                  data-testid="profile-suivi-btn"
                >
                  <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="currentColor">
                    <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41L9 16.17z"/>
                  </svg>
                  <span className="text-xs">Suivi</span>
                </button>
              </div>
              
              {/* Bio Section (Scrollable) */}
              {selectedFollower.bio && (
                <div className="flex-1 overflow-y-auto px-6 pb-6">
                  <div className="bg-white/5 rounded-xl p-4">
                    <h4 className="text-white/50 text-xs uppercase tracking-wide mb-2">Bio</h4>
                    <p className="text-white/80 text-sm leading-relaxed">{selectedFollower.bio}</p>
                  </div>
                </div>
              )}
            </div>
          </div>
        )}

        {/* ===== FULL FOLLOWERS VIEW ===== */}
        {showAllFollowers && (
          <div className="absolute inset-0 bg-black z-40 flex flex-col">
            <div className="flex-shrink-0 flex items-center justify-between px-4 py-3 border-b border-white/10">
              <button onClick={() => setShowAllFollowers(false)} className="text-white/70 hover:text-white">
                <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
                </svg>
              </button>
              <h1 className="text-white font-bold text-lg">Mes Bros.</h1>
              <div className="w-6"></div>
            </div>
            
            <div className="flex-shrink-0 px-4 py-3">
              <div className="relative">
                <svg className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-white/40" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <circle cx="11" cy="11" r="8" />
                  <path d="m21 21-4.35-4.35" />
                </svg>
                <input
                  type="text"
                  placeholder="Rechercher un pseudo..."
                  value={followerSearchQuery}
                  onChange={(e) => setFollowerSearchQuery(e.target.value)}
                  className="w-full bg-white/10 border border-white/10 rounded-full py-2.5 pl-10 pr-4 text-white text-sm placeholder:text-white/40 focus:outline-none focus:border-white/30"
                />
              </div>
            </div>
            
            <div className="flex-1 overflow-y-auto px-4 pb-6">
              {filteredFollowers.length === 0 ? (
                <div className="flex flex-col items-center justify-center py-12">
                  <p className="text-white/40 text-sm">Aucun follower trouvé</p>
                </div>
              ) : (
                <div className="grid grid-cols-4 gap-3">
                  {filteredFollowers.map((followedUser) => (
                    <button 
                      key={followedUser.id} 
                      onClick={() => { handleFollowerClick(followedUser); setShowAllFollowers(false); }}
                      className="flex flex-col items-center gap-1.5"
                    >
                      <div className="relative">
                        <div className="w-16 h-16 rounded-full border-2 border-white overflow-hidden">
                          {followedUser.profile_photo ? (
                            <img src={followedUser.profile_photo} alt="" className="w-full h-full object-cover" />
                          ) : (
                            <div className="w-full h-full bg-white/10 flex items-center justify-center">
                              <svg className="w-5 h-5 text-white/70" viewBox="0 0 24 24" fill="currentColor">
                                <path d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"/>
                              </svg>
                            </div>
                          )}
                        </div>
                        {followedUser.is_online && (
                          <div className="absolute bottom-0 right-0 w-4 h-4 bg-green-500 border-2 border-black rounded-full"></div>
                        )}
                      </div>
                      <span className="text-white/80 text-[11px] font-medium truncate max-w-[64px]">
                        @{followedUser.display_name || 'User'}
                      </span>
                    </button>
                  ))}
                </div>
              )}
            </div>
          </div>
        )}

        {/* ===== ADD BRO MODAL ===== */}
        {showAddBro && (
          <div className="absolute inset-0 bg-black z-40 flex flex-col">
            <div className="flex-shrink-0 flex items-center justify-between px-4 py-3 border-b border-white/10">
              <button onClick={() => setShowAddBro(false)} className="text-white/70 hover:text-white">
                <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
                </svg>
              </button>
              <h1 className="text-white font-bold text-lg">Ajouter un Bro</h1>
              <div className="w-6"></div>
            </div>

            <div className="flex-shrink-0 px-4 py-3">
              <div className="relative">
                <svg className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-white/40" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <circle cx="11" cy="11" r="8" />
                  <path d="m21 21-4.35-4.35" />
                </svg>
                <input
                  type="text"
                  placeholder="Rechercher un pseudo..."
                  value={addBroQuery}
                  onChange={(e) => setAddBroQuery(e.target.value)}
                  autoFocus
                  className="w-full bg-white/10 border border-white/10 rounded-full py-2.5 pl-10 pr-4 text-white text-sm placeholder:text-white/40 focus:outline-none focus:border-white/30"
                />
              </div>
            </div>

            <div className="flex-1 overflow-y-auto px-4 pb-6">
              {!addBroQuery.trim() ? (
                <div className="flex flex-col items-center justify-center py-12">
                  <p className="text-white/40 text-sm">Tape un pseudo pour chercher</p>
                </div>
              ) : addBroLoading ? (
                <div className="flex justify-center py-12">
                  <div className="w-6 h-6 border-2 border-white/30 border-t-white rounded-full animate-spin"></div>
                </div>
              ) : addBroResults.length === 0 ? (
                <div className="flex flex-col items-center justify-center py-12">
                  <p className="text-white/40 text-sm">Aucun utilisateur trouvé</p>
                </div>
              ) : (
                <div className="grid grid-cols-3 gap-4">
                  {addBroResults.map((broUser) => {
                    const isFollowing = following.some(f => f.id === broUser.id);
                    const isPending = pendingFollows.has(broUser.id);
                    return (
                      <div key={broUser.id} className="flex flex-col items-center gap-2">
                        <div className="relative">
                          <div className="w-16 h-16 rounded-full border-2 border-white overflow-hidden">
                            {broUser.profile_photo ? (
                              <img src={broUser.profile_photo} alt="" className="w-full h-full object-cover" />
                            ) : (
                              <div className="w-full h-full bg-white/10 flex items-center justify-center">
                                <svg className="w-5 h-5 text-white/70" viewBox="0 0 24 24" fill="currentColor">
                                  <path d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"/>
                                </svg>
                              </div>
                            )}
                          </div>
                          {broUser.is_online && (
                            <div className="absolute bottom-0 right-0 w-4 h-4 bg-green-500 border-2 border-black rounded-full"></div>
                          )}
                        </div>
                        <span className="text-white/80 text-[11px] font-medium truncate max-w-[80px]">
                          @{broUser.display_name || 'User'}
                        </span>
                        {isFollowing ? (
                          <span className="text-[11px] text-white/30 font-medium">Suivi</span>
                        ) : isPending ? (
                          <span className="text-[11px] text-white/50 font-medium">En attente</span>
                        ) : (
                          <button
                            onClick={() => handleAddBroFollow(broUser.id)}
                            className="px-3 py-1 bg-white text-black text-[11px] font-bold rounded-full hover:bg-white/90 transition-colors"
                          >
                            Suivre
                          </button>
                        )}
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          </div>
        )}

        {/* ===== NEW MESSAGE MODAL ===== */}
        {showNewMessage && (
          <div className="absolute inset-0 bg-black z-40 flex flex-col">
            <div className="flex-shrink-0 flex items-center justify-between px-4 py-3 border-b border-white/10">
              <button onClick={() => setShowNewMessage(false)} className="text-white/70 hover:text-white">
                <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
              <h1 className="text-white font-bold text-lg">Nouveau message</h1>
              <button 
                onClick={startConversation}
                disabled={selectedContacts.length === 0}
                className={`text-sm font-bold ${selectedContacts.length > 0 ? 'text-white' : 'text-white/30'}`}
              >
                Chatter
              </button>
            </div>
            
            {selectedContacts.length > 0 && (
              <div className="flex-shrink-0 px-4 py-3 border-b border-white/10">
                <div className="flex gap-2 overflow-x-auto scrollbar-hide">
                  {selectedContacts.map((contact) => (
                    <div key={contact.id} className="flex items-center gap-1.5 bg-white/10 rounded-full pl-1 pr-2 py-1 flex-shrink-0">
                      <div className="w-6 h-6 rounded-full border border-white overflow-hidden">
                        {contact.profile_photo ? (
                          <img src={contact.profile_photo} alt="" className="w-full h-full object-cover" />
                        ) : (
                          <div className="w-full h-full bg-white/10 flex items-center justify-center">
                            <svg className="w-3 h-3 text-white/70" viewBox="0 0 24 24" fill="currentColor">
                              <path d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"/>
                            </svg>
                          </div>
                        )}
                      </div>
                      <span className="text-white text-xs">@{contact.display_name}</span>
                      <button onClick={() => toggleContactSelection(contact)} className="text-white/50 hover:text-white">
                        <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2">
                          <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
                        </svg>
                      </button>
                    </div>
                  ))}
                </div>
              </div>
            )}
            
            <div className="flex-shrink-0 px-4 py-3">
              <div className="relative">
                <svg className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-white/40" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <circle cx="11" cy="11" r="8" />
                  <path d="m21 21-4.35-4.35" />
                </svg>
                <input
                  type="text"
                  placeholder="Rechercher un follower..."
                  value={contactSearchQuery}
                  onChange={(e) => setContactSearchQuery(e.target.value)}
                  className="w-full bg-white/10 border border-white/10 rounded-full py-2.5 pl-10 pr-4 text-white text-sm placeholder:text-white/40 focus:outline-none focus:border-white/30"
                />
              </div>
            </div>
            
            <div className="flex-shrink-0 px-4 pb-2">
              <p className="text-white/40 text-xs">Sélectionne un ou plusieurs followers pour créer une conversation</p>
            </div>
            
            <div className="flex-1 overflow-y-auto px-4 pb-6">
              {filteredContacts.length === 0 ? (
                <div className="flex flex-col items-center justify-center py-12">
                  <p className="text-white/40 text-sm">Aucun follower trouvé</p>
                </div>
              ) : (
                <div className="space-y-1">
                  {filteredContacts.map((contact) => {
                    const isSelected = selectedContacts.find(c => c.id === contact.id);
                    return (
                      <button 
                        key={contact.id}
                        onClick={() => toggleContactSelection(contact)}
                        className={`w-full flex items-center gap-3 p-3 rounded-xl transition-all ${isSelected ? 'bg-white/20' : 'hover:bg-white/10'}`}
                      >
                        <div className="relative">
                          <div className="w-12 h-12 rounded-full border-2 border-white overflow-hidden">
                            {contact.profile_photo ? (
                              <img src={contact.profile_photo} alt="" className="w-full h-full object-cover" />
                            ) : (
                              <div className="w-full h-full bg-white/10 flex items-center justify-center">
                                <svg className="w-5 h-5 text-white/70" viewBox="0 0 24 24" fill="currentColor">
                                  <path d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"/>
                                </svg>
                              </div>
                            )}
                          </div>
                          {contact.is_online && (
                            <div className="absolute bottom-0 right-0 w-3 h-3 bg-green-500 border-2 border-black rounded-full"></div>
                          )}
                        </div>
                        <div className="flex-1 text-left">
                          <span className="text-white font-bold text-sm">@{contact.display_name}</span>
                        </div>
                        <div className={`w-6 h-6 rounded-full border-2 flex items-center justify-center ${isSelected ? 'bg-white border-white' : 'border-white/30'}`}>
                          {isSelected && (
                            <svg className="w-4 h-4 text-black" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="3">
                              <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
                            </svg>
                          )}
                        </div>
                      </button>
                    );
                  })}
                </div>
              )}
            </div>
          </div>
        )}

        {/* ===== CONVERSATION VIEW ===== */}
        {activeConversation && (
          <div className="absolute inset-0 bg-black z-40 flex flex-col">
            {/* Header */}
            <div className="flex-shrink-0 flex items-center justify-between px-4 py-3 border-b border-white/10">
              <button onClick={() => setActiveConversation(null)} className="text-white/70 hover:text-white">
                <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
                </svg>
              </button>
              
              {/* Partner Info - Clickable to open profile or group modal */}
              <button 
                onClick={() => {
                  if (activeConversation.is_group) {
                    // Open group management modal
                    fetchGroupMembers(activeConversation.id);
                    setShowGroupModal(true);
                  } else {
                    const partner = following.find(f => f.id === activeConversation.partner_id || f.credential_id === activeConversation.partner_id);
                    if (partner) {
                      setSelectedFollower(partner);
                    }
                  }
                }}
                className="flex items-center gap-2 hover:bg-white/5 rounded-lg px-2 py-1 transition-all"
              >
                <div className="relative">
                  <div className="w-8 h-8 rounded-full border-2 border-white overflow-hidden">
                    {activeConversation.is_group ? (
                      activeConversation.group_photo ? (
                        <img src={activeConversation.group_photo} alt="" className="w-full h-full object-cover" />
                      ) : (
                        <div className="w-full h-full bg-gradient-to-br from-purple-500 to-blue-500 flex items-center justify-center">
                          <svg className="w-4 h-4 text-white" viewBox="0 0 24 24" fill="currentColor">
                            <path d="M12 6c1.1 0 2 .9 2 2s-.9 2-2 2-2-.9-2-2 .9-2 2-2m0 10c2.7 0 5.8 1.29 6 2H6c.23-.72 3.31-2 6-2m0-12C9.79 4 8 5.79 8 8s1.79 4 4 4 4-1.79 4-4-1.79-4-4-4zm0 10c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"/>
                          </svg>
                        </div>
                      )
                    ) : activeConversation.partner_photo ? (
                      <img src={activeConversation.partner_photo} alt="" className="w-full h-full object-cover" />
                    ) : (
                      <div className="w-full h-full bg-white/10 flex items-center justify-center">
                        <svg className="w-4 h-4 text-white/70" viewBox="0 0 24 24" fill="currentColor">
                          <path d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"/>
                        </svg>
                      </div>
                    )}
                  </div>
                  {!activeConversation.is_group && activeConversation.partner_online && (
                    <div className="absolute bottom-0 right-0 w-2.5 h-2.5 bg-green-500 border-2 border-black rounded-full"></div>
                  )}
                </div>
                <span className="text-white font-bold text-sm">
                  {activeConversation.is_group
                    ? (activeConversation.group_name || activeConversation.name || 'Groupe')
                    : `@${activeConversation.partner_name}`}
                </span>
                {/* Group info icon */}
                {activeConversation.is_group && (
                  <svg className="w-4 h-4 text-white/50" viewBox="0 0 24 24" fill="currentColor">
                    <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-6h2v6zm0-8h-2V7h2v2z"/>
                  </svg>
                )}
              </button>
              
              {/* Video Call Button - Only show for 1-on-1 conversations (not groups) */}
              {!activeConversation.is_group && (
                <button
                  onClick={() => {
                    startCall(
                      activeConversation.partner_id,
                      activeConversation.partner_name,
                      activeConversation.partner_photo,
                      activeConversation.partner_online
                    );
                  }}
                  className={`w-9 h-9 flex items-center justify-center rounded-full ${activeConversation.partner_online === false ? 'bg-white/30 cursor-not-allowed' : 'bg-white/10'}`}
                  data-testid="conversation-call-btn"
                >
                  <svg className="w-5 h-5 text-white" fill="currentColor" viewBox="0 0 24 24">
                    <path d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
                  </svg>
                </button>
              )}
              {/* Placeholder for group chats to maintain header alignment */}
              {activeConversation.is_group && <div className="w-9 h-9"></div>}
            </div>
            
            {/* Messages */}
            <div className="flex-1 overflow-y-auto px-4 py-4">
              {conversationMessages.length === 0 ? (
                <div className="flex flex-col items-center justify-center h-full">
                  <p className="text-white/30 text-sm">Commencez la conversation</p>
                </div>
              ) : (
                <div className="space-y-3">
                  {conversationMessages.map((msg, index) => {
                    const isOwn = msg.sender_id === (user?.credential_id || user?.id);
                    const msgDate = new Date(msg.created_at);
                    const prevMsg = conversationMessages[index - 1];
                    const prevDate = prevMsg ? new Date(prevMsg.created_at) : null;
                    
                    // Show date separator if day changed
                    const showDateSeparator = !prevDate || 
                      msgDate.toDateString() !== prevDate.toDateString();
                    
                    return (
                      <React.Fragment key={msg.id}>
                        {showDateSeparator && (
                          <div className="flex justify-center my-3">
                            <span className="text-white/30 text-[10px] bg-white/5 px-3 py-1 rounded-full">
                              {msgDate.toLocaleDateString('fr-FR', { weekday: 'long', day: 'numeric', month: 'long' })}
                            </span>
                          </div>
                        )}
                        <div className={`flex items-end gap-1 group ${isOwn ? 'justify-end' : 'justify-start'}`}>
                          {/* Report button for received messages */}
                          {!isOwn && (
                            <button 
                              onClick={() => handleReportClick('message', msg.id, msg.sender_id, activeConversation.partner_name)}
                              className="w-6 h-6 flex items-center justify-center text-white/20 hover:text-white/60 transition-colors opacity-0 group-hover:opacity-100"
                            >
                              <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="currentColor">
                                <path d="M12 2L4 5v6.09c0 5.05 3.41 9.76 8 10.91 4.59-1.15 8-5.86 8-10.91V5l-8-3zm0 15c-.55 0-1-.45-1-1v-1c0-.55.45-1 1-1s1 .45 1 1v1c0 .55-.45 1-1 1zm1-4h-2V8h2v5z"/>
                              </svg>
                            </button>
                          )}
                          <div className={`max-w-[75%] rounded-2xl overflow-hidden ${isOwn ? 'bg-white text-black' : 'bg-white/10 text-white'}`}>
                            {/* Media content */}
                            {msg.media_url && (() => {
                              const isPrivateMedia = msg.is_private && !isOwn && !revealedMessages.has(msg.id);
                              return (
                              <div className="relative">
                                {msg.media_type?.startsWith('image/') ? (
                                  <>
                                    <img
                                      src={msg.media_url}
                                      alt="Media"
                                      className={`w-full max-h-[300px] object-cover ${isPrivateMedia ? 'blur-xl' : 'cursor-pointer'}`}
                                      onClick={() => !isPrivateMedia && window.open(msg.media_url, '_blank')}
                                    />
                                    {isPrivateMedia && (
                                      <div className="absolute inset-0 flex flex-col items-center justify-center bg-black/30">
                                        <span className="text-2xl mb-2">🔒</span>
                                        <p className="text-white text-xs font-medium mb-2">Photo privée</p>
                                        <button
                                          onClick={() => setRevealedMessages(prev => new Set([...prev, msg.id]))}
                                          className="px-4 py-1.5 bg-white text-black text-xs font-bold rounded-full hover:bg-white/90 transition-colors"
                                        >
                                          Voir
                                        </button>
                                      </div>
                                    )}
                                  </>
                                ) : msg.media_type?.startsWith('video/') ? (
                                  <>
                                    {isPrivateMedia ? (
                                      <div className="w-full h-[200px] bg-black/60 flex flex-col items-center justify-center">
                                        <span className="text-2xl mb-2">🔒</span>
                                        <p className="text-white text-xs font-medium mb-2">Vidéo privée</p>
                                        <button
                                          onClick={() => setRevealedMessages(prev => new Set([...prev, msg.id]))}
                                          className="px-4 py-1.5 bg-white text-black text-xs font-bold rounded-full hover:bg-white/90 transition-colors"
                                        >
                                          Voir
                                        </button>
                                      </div>
                                    ) : (
                                      <video
                                        src={msg.media_url}
                                        controls
                                        className="w-full max-h-[300px]"
                                      />
                                    )}
                                  </>
                                ) : (
                                  <div className="p-4 flex items-center gap-2">
                                    <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z" />
                                    </svg>
                                    <span className="text-sm">Fichier</span>
                                  </div>
                                )}
                                {msg.uploading && (
                                  <div className="absolute inset-0 bg-black/50 flex items-center justify-center">
                                    <div className="w-8 h-8 border-2 border-white border-t-transparent rounded-full animate-spin" />
                                  </div>
                                )}
                              </div>
                              );
                            })()}
                            {/* Text content */}
                            {msg.content && msg.content !== '[Média]' && msg.content !== '[Image]' && (
                              <div className="px-4 py-2.5">
                                <p className="text-sm">{msg.content}</p>
                              </div>
                            )}
                            {/* Timestamp */}
                            <div className={`px-4 pb-2 ${msg.media_url && (!msg.content || msg.content === '[Média]' || msg.content === '[Image]') ? 'pt-2' : ''}`}>
                              <p className={`text-[10px] ${isOwn ? 'text-black/50' : 'text-white/40'}`}>
                                {msgDate.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })}
                              </p>
                            </div>
                          </div>
                        </div>
                      </React.Fragment>
                    );
                  })}
                  <div ref={messagesEndRef} />
                </div>
              )}
            </div>
            
            {/* Message Input */}
            <div className="flex-shrink-0 px-4 py-3 border-t border-white/10">
              {/* Not Bros Warning */}
              {activeConversation.not_bros ? (
                <div className="text-center py-2">
                  <p className="text-white/50 text-sm">Vous n&apos;êtes plus mutuellement suivis</p>
                  <p className="text-white/30 text-xs mt-1">Les messages ne peuvent plus être envoyés</p>
                </div>
              ) : (
                <>
                  {/* Hidden file input */}
                  <input
                    type="file"
                    ref={mediaInputRef}
                    onChange={handleMediaUpload}
                    accept="image/*,video/*"
                    className="hidden"
                  />
                  <div className="flex items-center gap-2">
                    {/* Media Button */}
                    <button
                      onClick={() => mediaInputRef.current?.click()}
                      className="w-10 h-10 flex items-center justify-center bg-white/10 rounded-full flex-shrink-0 hover:bg-white/20 transition-colors"
                    >
                      <svg className="w-5 h-5 text-white" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                        <path strokeLinecap="round" strokeLinejoin="round" d="M12 4v16m8-8H4" />
                      </svg>
                    </button>

                    {/* Single View Toggle */}
                    <button
                      onClick={() => setSingleViewEnabled(!singleViewEnabled)}
                      title="Vue unique"
                      className={`w-10 h-10 flex items-center justify-center rounded-full flex-shrink-0 transition-colors ${singleViewEnabled ? 'bg-white text-black' : 'bg-white/10 text-white/60 hover:bg-white/20'}`}
                    >
                      <svg className="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                        <path strokeLinecap="round" strokeLinejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                        <path strokeLinecap="round" strokeLinejoin="round" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
                      </svg>
                    </button>

                    {/* Input */}
                    <input
                      type="text"
                      placeholder="Message..."
                      value={newMessageText}
                      onChange={(e) => setNewMessageText(e.target.value)}
                      onKeyPress={(e) => e.key === 'Enter' && sendMessage()}
                      className="flex-1 bg-white/10 border border-white/10 rounded-full py-2.5 px-4 text-white text-sm placeholder:text-white/40 focus:outline-none focus:border-white/30"
                    />

                    {/* Close keyboard Button */}
                    <button
                      onClick={() => document.activeElement?.blur()}
                      className="w-8 h-8 flex items-center justify-center text-white/40 hover:text-white/70 transition-colors flex-shrink-0"
                    >
                      <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2.5">
                        <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
                      </svg>
                    </button>

                    {/* Send Button */}
                    <button
                      onClick={sendMessage}
                      disabled={!newMessageText.trim()}
                      className={`w-10 h-10 flex items-center justify-center rounded-full flex-shrink-0 ${newMessageText.trim() ? 'bg-white' : 'bg-white/10'}`}
                    >
                      <svg className={`w-5 h-5 ${newMessageText.trim() ? 'text-black' : 'text-white/40'}`} viewBox="0 0 24 24" fill="currentColor">
                        <path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z" />
                      </svg>
                    </button>
                  </div>
                </>
              )}
            </div>
          </div>
        )}
      </div>
      
      {/* Group Management Modal */}
      {showGroupModal && activeConversation?.is_group && (
        <div className="fixed inset-0 bg-black/80 z-[9999] flex items-center justify-center p-4 animate-fade-in">
          <div className="bg-zinc-900 rounded-2xl w-full max-w-md max-h-[80vh] flex flex-col overflow-hidden border border-white/10">
            {/* Header */}
            <div className="p-4 border-b border-white/10 flex items-center justify-between">
              <h2 className="text-white font-bold text-lg">Infos du groupe</h2>
              <button 
                onClick={() => {
                  setShowGroupModal(false);
                  setIsEditingGroupName(false);
                }}
                className="text-white/50 hover:text-white"
              >
                <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>
            
            {/* Group Photo & Name Section */}
            <div className="p-4 border-b border-white/10">
              <div className="flex items-center gap-4">
                {/* Group Photo with edit button */}
                <div className="relative flex-shrink-0">
                  <input
                    type="file"
                    ref={groupPhotoInputRef}
                    onChange={handleGroupPhotoUpload}
                    accept="image/*"
                    className="hidden"
                  />
                  <button
                    onClick={() => groupPhotoInputRef.current?.click()}
                    className="w-16 h-16 rounded-full overflow-hidden border-2 border-white/20 hover:border-purple-500 transition-all group"
                  >
                    {groupPhoto ? (
                      <img src={groupPhoto} alt="" className="w-full h-full object-cover" />
                    ) : (
                      <div className="w-full h-full bg-gradient-to-br from-purple-500 to-blue-500 flex items-center justify-center">
                        <svg className="w-8 h-8 text-white" viewBox="0 0 24 24" fill="currentColor">
                          <path d="M12 6c1.1 0 2 .9 2 2s-.9 2-2 2-2-.9-2-2 .9-2 2-2m0 10c2.7 0 5.8 1.29 6 2H6c.23-.72 3.31-2 6-2m0-12C9.79 4 8 5.79 8 8s1.79 4 4 4 4-1.79 4-4-1.79-4-4-4zm0 10c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"/>
                        </svg>
                      </div>
                    )}
                    {/* Hover overlay */}
                    <div className="absolute inset-0 bg-black/50 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center rounded-full">
                      <svg className="w-6 h-6 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 9a2 2 0 012-2h.93a2 2 0 001.664-.89l.812-1.22A2 2 0 0110.07 4h3.86a2 2 0 011.664.89l.812 1.22A2 2 0 0018.07 7H19a2 2 0 012 2v9a2 2 0 01-2 2H5a2 2 0 01-2-2V9z" />
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 13a3 3 0 11-6 0 3 3 0 016 0z" />
                      </svg>
                    </div>
                  </button>
                </div>
                <div className="flex-1">
                  {isEditingGroupName ? (
                    <div className="flex items-center gap-2">
                      <input
                        type="text"
                        value={newGroupName}
                        onChange={(e) => setNewGroupName(e.target.value)}
                        placeholder="Nom du groupe"
                        className="flex-1 bg-white/10 text-white rounded-lg px-3 py-2 text-sm border border-white/20 focus:border-purple-500 focus:outline-none"
                        maxLength={50}
                        autoFocus
                      />
                      <button
                        onClick={handleRenameGroup}
                        className="p-2 bg-purple-600 rounded-lg text-white hover:bg-purple-500"
                      >
                        <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                        </svg>
                      </button>
                      <button
                        onClick={() => {
                          setIsEditingGroupName(false);
                          setNewGroupName(groupName);
                        }}
                        className="p-2 bg-white/10 rounded-lg text-white hover:bg-white/20"
                      >
                        <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                        </svg>
                      </button>
                    </div>
                  ) : (
                    <div className="flex items-center gap-2">
                      <span className="text-white font-bold text-lg">
                        {groupName || `Groupe (${groupMembers.length})`}
                      </span>
                      <button
                        onClick={() => setIsEditingGroupName(true)}
                        className="p-1.5 bg-white/10 rounded-lg text-white/70 hover:text-white hover:bg-white/20"
                      >
                        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
                        </svg>
                      </button>
                    </div>
                  )}
                  <p className="text-white/50 text-sm mt-1">{groupMembers.length} membres</p>
                </div>
              </div>
            </div>
            
            {/* Members List */}
            <div className="flex-1 overflow-y-auto p-4">
              <h3 className="text-white/70 text-sm font-medium mb-3">Membres</h3>
              <div className="space-y-2">
                {groupMembers.map((member) => (
                  <div key={member.id} className="flex items-center gap-3 p-2 rounded-lg hover:bg-white/5">
                    <div className="relative">
                      <div className="w-10 h-10 rounded-full border-2 border-white/20 overflow-hidden">
                        {member.profile_photo ? (
                          <img src={member.profile_photo} alt="" className="w-full h-full object-cover" />
                        ) : (
                          <div className="w-full h-full bg-white/10 flex items-center justify-center">
                            <svg className="w-5 h-5 text-white/50" viewBox="0 0 24 24" fill="currentColor">
                              <path d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"/>
                            </svg>
                          </div>
                        )}
                      </div>
                      {member.is_online && (
                        <div className="absolute bottom-0 right-0 w-3 h-3 bg-green-500 border-2 border-zinc-900 rounded-full"></div>
                      )}
                    </div>
                    <div className="flex-1">
                      <p className="text-white font-medium text-sm">@{member.display_name}</p>
                      {member.country && (
                        <p className="text-white/40 text-xs">{member.country}</p>
                      )}
                    </div>
                    <span className={`text-xs ${member.is_online ? 'text-green-400' : 'text-white/30'}`}>
                      {member.is_online ? 'En ligne' : 'Hors ligne'}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Media Send Modal - Normal vs Private */}
      {showMediaModal && pendingMedia && (
        <div
          className="fixed inset-0 bg-black/80 z-[9999] flex items-center justify-center p-4 animate-fade-in"
          onClick={(e) => { if (e.target === e.currentTarget) { setShowMediaModal(false); setPendingMedia(null); }}}
        >
          <div className="bg-zinc-900 rounded-2xl w-full max-w-sm overflow-hidden border border-white/10">
            {/* Header */}
            <div className="p-4 border-b border-white/10 flex items-center justify-between">
              <h3 className="text-white font-bold text-base">Envoyer le média</h3>
              <button
                onClick={() => { setShowMediaModal(false); setPendingMedia(null); }}
                className="text-white/50 hover:text-white"
              >
                <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2">
                  <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>

            {/* Preview */}
            <div className="p-4">
              {pendingMedia.isImage && pendingMedia.previewUrl ? (
                <img src={pendingMedia.previewUrl} alt="Preview" className="w-full max-h-[250px] object-contain rounded-lg" />
              ) : pendingMedia.isVideo ? (
                <div className="w-full h-[150px] bg-white/5 rounded-lg flex items-center justify-center">
                  <svg className="w-12 h-12 text-white/30" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="1.5">
                    <path strokeLinecap="round" strokeLinejoin="round" d="M15.75 10.5l4.72-4.72a.75.75 0 011.28.53v11.38a.75.75 0 01-1.28.53l-4.72-4.72M4.5 18.75h9a2.25 2.25 0 002.25-2.25v-9a2.25 2.25 0 00-2.25-2.25h-9A2.25 2.25 0 002.25 7.5v9a2.25 2.25 0 002.25 2.25z" />
                  </svg>
                </div>
              ) : null}
              <p className="text-white/40 text-xs mt-2 text-center truncate">{pendingMedia.file.name}</p>
            </div>

            {/* Actions */}
            <div className="p-4 pt-0 flex flex-col gap-2">
              <button
                onClick={() => sendMedia(pendingMedia.file, false)}
                className="w-full py-3 bg-white text-black rounded-full font-bold text-sm hover:bg-white/90 transition-colors"
              >
                Envoyer
              </button>
              <button
                onClick={() => sendMedia(pendingMedia.file, true)}
                className="w-full py-3 bg-white/10 text-white rounded-full font-bold text-sm hover:bg-white/20 transition-colors flex items-center justify-center gap-2"
              >
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2">
                  <path strokeLinecap="round" strokeLinejoin="round" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
                </svg>
                Envoyer en privé
              </button>
            </div>
          </div>
        </div>
      )}

      {/* CSS for animations */}
      <style>{`
        @keyframes slide-up {
          from { transform: translateY(100%); }
          to { transform: translateY(0); }
        }
        .animate-slide-up {
          animation: slide-up 0.3s ease-out;
        }
        @keyframes fade-in {
          from { opacity: 0; }
          to { opacity: 1; }
        }
        .animate-fade-in {
          animation: fade-in 0.3s ease-out;
        }
        @keyframes pulse-ring {
          0% { box-shadow: 0 0 0 0 rgba(34, 197, 94, 0.7); }
          70% { box-shadow: 0 0 0 10px rgba(34, 197, 94, 0); }
          100% { box-shadow: 0 0 0 0 rgba(34, 197, 94, 0); }
        }
        .animate-pulse-ring {
          animation: pulse-ring 2s infinite;
        }
      `}</style>
      {/* Account Settings Modal */}
      <AccountSettings isOpen={showAccountSettings} onClose={() => setShowAccountSettings(false)} />
    </div>
  );
};

export default Space;


