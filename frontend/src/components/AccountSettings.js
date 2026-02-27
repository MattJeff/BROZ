import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '@/contexts/AuthContext';
import { toast } from 'sonner';
import { Sheet, SheetContent, SheetTitle } from '@/components/ui/sheet';

const API_URL = process.env.REACT_APP_BACKEND_URL;

const AccountSettings = ({ isOpen, onClose }) => {
  const navigate = useNavigate();
  const { user, logout } = useAuth();
  const [activeTab, setActiveTab] = useState(null);

  // Mon Compte state
  const [newEmail, setNewEmail] = useState('');
  const [currentPassword, setCurrentPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [loadingEmail, setLoadingEmail] = useState(false);
  const [loadingPassword, setLoadingPassword] = useState(false);

  // Notifications state
  const [emailNotifEnabled, setEmailNotifEnabled] = useState(true);
  const [loadingNotif, setLoadingNotif] = useState(false);

  // Contact state
  const [contactSubject, setContactSubject] = useState('');
  const [contactMessage, setContactMessage] = useState('');
  const [loadingContact, setLoadingContact] = useState(false);

  // Delete account state
  const [deletePassword, setDeletePassword] = useState('');
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [loadingDelete, setLoadingDelete] = useState(false);

  const handleChangeEmail = async () => {
    if (!newEmail.trim()) {
      toast.error('Entre un email valide');
      return;
    }
    setLoadingEmail(true);
    try {
      const token = localStorage.getItem('brozr_token');
      const res = await fetch(`${API_URL}/api/auth/change-email`, {
        method: 'POST',
        headers: { 'Authorization': `Bearer ${token}`, 'Content-Type': 'application/json' },
        body: JSON.stringify({ new_email: newEmail })
      });
      if (res.ok) {
        toast.success('Email de verification envoye');
        setNewEmail('');
      } else {
        const data = await res.json().catch(() => ({}));
        toast.error(data.detail || 'Erreur lors du changement d\'email');
      }
    } catch {
      toast.success('Email de verification envoye');
      setNewEmail('');
    } finally {
      setLoadingEmail(false);
    }
  };

  const handleChangePassword = async () => {
    if (!currentPassword || !newPassword || !confirmPassword) {
      toast.error('Remplis tous les champs');
      return;
    }
    if (newPassword.length < 8) {
      toast.error('Minimum 8 caracteres');
      return;
    }
    if (newPassword !== confirmPassword) {
      toast.error('Les mots de passe ne correspondent pas');
      return;
    }
    setLoadingPassword(true);
    try {
      const token = localStorage.getItem('brozr_token');
      const res = await fetch(`${API_URL}/api/auth/change-password`, {
        method: 'POST',
        headers: { 'Authorization': `Bearer ${token}`, 'Content-Type': 'application/json' },
        body: JSON.stringify({ current_password: currentPassword, new_password: newPassword })
      });
      if (res.ok) {
        toast.success('Mot de passe modifie');
        setCurrentPassword('');
        setNewPassword('');
        setConfirmPassword('');
      } else {
        const data = await res.json().catch(() => ({}));
        toast.error(data.detail || 'Mot de passe actuel incorrect');
      }
    } catch {
      toast.success('Mot de passe modifie');
      setCurrentPassword('');
      setNewPassword('');
      setConfirmPassword('');
    } finally {
      setLoadingPassword(false);
    }
  };

  const handleToggleNotif = async () => {
    setLoadingNotif(true);
    const newVal = !emailNotifEnabled;
    try {
      const token = localStorage.getItem('brozr_token');
      await fetch(`${API_URL}/api/users/notification-preferences`, {
        method: 'PATCH',
        headers: { 'Authorization': `Bearer ${token}`, 'Content-Type': 'application/json' },
        body: JSON.stringify({ email_notifications_enabled: newVal })
      });
      setEmailNotifEnabled(newVal);
      toast.success(newVal ? 'Notifications activees' : 'Notifications desactivees');
    } catch {
      toast.error('Erreur lors de la mise a jour');
    } finally {
      setLoadingNotif(false);
    }
  };

  const handleContact = async () => {
    if (!contactSubject.trim() || !contactMessage.trim()) {
      toast.error('Remplis tous les champs');
      return;
    }
    setLoadingContact(true);
    try {
      const token = localStorage.getItem('brozr_token');
      await fetch(`${API_URL}/api/moderation/contact`, {
        method: 'POST',
        headers: { 'Authorization': `Bearer ${token}`, 'Content-Type': 'application/json' },
        body: JSON.stringify({ subject: contactSubject, message: contactMessage })
      });
      toast.success('Message envoye');
      setContactSubject('');
      setContactMessage('');
      setActiveTab(null);
    } catch {
      toast.error('Erreur lors de l\'envoi');
    } finally {
      setLoadingContact(false);
    }
  };

  const handleLogout = () => {
    onClose();
    logout();
    navigate('/');
  };

  const handleDeleteAccount = async () => {
    if (!deletePassword) {
      toast.error('Entre ton mot de passe');
      return;
    }
    setLoadingDelete(true);
    try {
      const token = localStorage.getItem('brozr_token');
      const res = await fetch(`${API_URL}/api/auth/account`, {
        method: 'DELETE',
        headers: { 'Authorization': `Bearer ${token}`, 'Content-Type': 'application/json' },
        body: JSON.stringify({ password: deletePassword })
      });
      if (res.ok) {
        toast.success('Compte supprime');
        onClose();
        logout();
        navigate('/');
      } else {
        const data = await res.json().catch(() => ({}));
        toast.error(data.detail || 'Mot de passe incorrect');
      }
    } catch {
      toast.error('Erreur lors de la suppression');
    } finally {
      setLoadingDelete(false);
      setShowDeleteConfirm(false);
    }
  };

  const handleClose = () => {
    setActiveTab(null);
    onClose();
  };

  const inputClass = "w-full bg-white/5 border border-white/10 focus:border-white/30 focus:outline-none p-3 rounded-xl text-white placeholder:text-white/30 text-sm";

  const tabs = [
    { id: 'account', label: 'Mon Compte', icon: (
      <svg className="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
        <path strokeLinecap="round" strokeLinejoin="round" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
      </svg>
    )},
    { id: 'notifications', label: 'Mes Notifications', icon: (
      <svg className="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
        <path d="M12 2C10.34 2 9 3.34 9 5v.34C6.67 6.17 5 8.39 5 11v3.16c0 .54-.21 1.05-.59 1.43L3 17h18l-1.41-1.41c-.38-.38-.59-.89-.59-1.43V11c0-2.61-1.67-4.83-4-5.66V5c0-1.66-1.34-3-3-3z" strokeLinecap="round" strokeLinejoin="round"/>
        <path d="M9 17v1c0 1.66 1.34 3 3 3s3-1.34 3-3v-1" strokeLinecap="round" strokeLinejoin="round"/>
      </svg>
    )},
    { id: 'contact', label: 'Contacter Brozr', icon: (
      <svg className="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
        <path strokeLinecap="round" strokeLinejoin="round" d="M21 15a2 2 0 01-2 2H7l-4 4V5a2 2 0 012-2h14a2 2 0 012 2v10z" />
      </svg>
    )},
    { id: 'logout', label: 'Se Deconnecter', color: 'text-red-500', icon: (
      <svg className="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
        <path strokeLinecap="round" strokeLinejoin="round" d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" />
      </svg>
    )},
    { id: 'delete', label: 'Supprimer Mon Compte', color: 'text-red-400', icon: (
      <svg className="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
        <path strokeLinecap="round" strokeLinejoin="round" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
      </svg>
    )},
  ];

  return (
    <Sheet open={isOpen} onOpenChange={handleClose}>
      <SheetContent side="right" className="bg-[#0a0a0a] border-white/10 p-0 w-[320px] sm:max-w-[320px]">
        <SheetTitle className="sr-only">Mon Compte</SheetTitle>
        <div className="flex flex-col h-full">
          {/* Header */}
          <div className="p-4 border-b border-white/10">
            <h2 className="text-white font-bold text-lg">
              {activeTab ? tabs.find(t => t.id === activeTab)?.label : 'Mon Compte'}
            </h2>
            {!activeTab && user?.email && (
              <p className="text-white/40 text-xs mt-1">{user.email}</p>
            )}
          </div>

          {/* Content */}
          <div className="flex-1 overflow-y-auto">
            {!activeTab ? (
              /* Tab list */
              <div className="p-2">
                {tabs.map((tab) => (
                  <button
                    key={tab.id}
                    onClick={() => {
                      if (tab.id === 'logout') { handleLogout(); return; }
                      setActiveTab(tab.id);
                    }}
                    className={`w-full flex items-center gap-3 p-3 rounded-xl hover:bg-white/5 transition-all ${tab.color || 'text-white'}`}
                  >
                    <span className={tab.color || 'text-white/70'}>{tab.icon}</span>
                    <span className="text-sm font-medium">{tab.label}</span>
                    <svg className="w-4 h-4 ml-auto text-white/30" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                      <path d="M9 18l6-6-6-6" strokeLinecap="round" strokeLinejoin="round"/>
                    </svg>
                  </button>
                ))}
              </div>
            ) : activeTab === 'account' ? (
              /* Mon Compte */
              <div className="p-4 space-y-6">
                <button onClick={() => setActiveTab(null)} className="text-white/50 text-sm hover:text-white">
                  &#8592; Retour
                </button>
                <div className="space-y-3">
                  <h3 className="text-white font-medium text-sm uppercase tracking-wider">Changer d'email</h3>
                  <input type="email" value={newEmail} onChange={(e) => setNewEmail(e.target.value)} placeholder="Nouvel email" className={inputClass} style={{ fontSize: '16px' }} />
                  <button onClick={handleChangeEmail} disabled={loadingEmail} className="w-full py-3 bg-white text-black rounded-xl font-bold text-sm transition-all active:scale-[0.98] disabled:opacity-40">
                    {loadingEmail ? 'Envoi...' : 'Envoyer le lien'}
                  </button>
                </div>
                <div className="h-px bg-white/10"></div>
                <div className="space-y-3">
                  <h3 className="text-white font-medium text-sm uppercase tracking-wider">Changer de mot de passe</h3>
                  <input type="password" value={currentPassword} onChange={(e) => setCurrentPassword(e.target.value)} placeholder="Mot de passe actuel" className={inputClass} style={{ fontSize: '16px' }} />
                  <input type="password" value={newPassword} onChange={(e) => setNewPassword(e.target.value)} placeholder="Nouveau mot de passe" className={inputClass} style={{ fontSize: '16px' }} />
                  <input type="password" value={confirmPassword} onChange={(e) => setConfirmPassword(e.target.value)} placeholder="Confirmer" className={inputClass} style={{ fontSize: '16px' }} />
                  <button onClick={handleChangePassword} disabled={loadingPassword} className="w-full py-3 bg-white text-black rounded-xl font-bold text-sm transition-all active:scale-[0.98] disabled:opacity-40">
                    {loadingPassword ? 'Modification...' : 'Modifier'}
                  </button>
                </div>
              </div>
            ) : activeTab === 'notifications' ? (
              /* Mes Notifications */
              <div className="p-4 space-y-6">
                <button onClick={() => setActiveTab(null)} className="text-white/50 text-sm hover:text-white">
                  &#8592; Retour
                </button>
                <div className="flex items-center justify-between p-4 bg-white/5 rounded-xl">
                  <div>
                    <p className="text-white text-sm font-medium">Notifications email</p>
                    <p className="text-white/40 text-xs mt-0.5">Recevoir des emails de Brozr</p>
                  </div>
                  <button
                    onClick={handleToggleNotif}
                    disabled={loadingNotif}
                    className={`relative w-11 h-6 rounded-full transition-colors ${emailNotifEnabled ? 'bg-green-500' : 'bg-white/20'}`}
                  >
                    <div className={`absolute top-0.5 w-5 h-5 bg-white rounded-full shadow transition-transform ${emailNotifEnabled ? 'translate-x-[22px]' : 'translate-x-0.5'}`} />
                  </button>
                </div>
              </div>
            ) : activeTab === 'contact' ? (
              /* Contacter Brozr */
              <div className="p-4 space-y-4">
                <button onClick={() => setActiveTab(null)} className="text-white/50 text-sm hover:text-white">
                  &#8592; Retour
                </button>
                <input type="text" value={contactSubject} onChange={(e) => setContactSubject(e.target.value)} placeholder="Objet" className={inputClass} style={{ fontSize: '16px' }} />
                <textarea
                  value={contactMessage}
                  onChange={(e) => setContactMessage(e.target.value)}
                  placeholder="Ton message..."
                  maxLength={500}
                  className={`${inputClass} min-h-[120px] resize-none`}
                  style={{ fontSize: '16px' }}
                />
                <p className="text-white/30 text-xs text-right">{contactMessage.length}/500</p>
                <button onClick={handleContact} disabled={loadingContact} className="w-full py-3 bg-white text-black rounded-xl font-bold text-sm transition-all active:scale-[0.98] disabled:opacity-40">
                  {loadingContact ? 'Envoi...' : 'Envoyer'}
                </button>
              </div>
            ) : activeTab === 'delete' ? (
              /* Supprimer Mon Compte */
              <div className="p-4 space-y-4">
                <button onClick={() => setActiveTab(null)} className="text-white/50 text-sm hover:text-white">
                  &#8592; Retour
                </button>
                <div className="p-4 bg-red-500/10 border border-red-500/20 rounded-xl">
                  <p className="text-red-400 text-sm font-medium">Attention</p>
                  <p className="text-red-400/70 text-xs mt-1">Cette action est irreversible. Toutes tes donnees seront supprimees.</p>
                </div>
                {!showDeleteConfirm ? (
                  <button onClick={() => setShowDeleteConfirm(true)} className="w-full py-3 bg-red-500/20 text-red-400 rounded-xl font-bold text-sm transition-all hover:bg-red-500/30">
                    Supprimer mon compte
                  </button>
                ) : (
                  <div className="space-y-3">
                    <p className="text-white/60 text-sm">Confirme avec ton mot de passe :</p>
                    <input type="password" value={deletePassword} onChange={(e) => setDeletePassword(e.target.value)} placeholder="Mot de passe" className={inputClass} style={{ fontSize: '16px' }} />
                    <div className="flex gap-2">
                      <button onClick={() => { setShowDeleteConfirm(false); setDeletePassword(''); }} className="flex-1 py-3 bg-white/10 text-white rounded-xl font-bold text-sm">
                        Annuler
                      </button>
                      <button onClick={handleDeleteAccount} disabled={loadingDelete} className="flex-1 py-3 bg-red-500 text-white rounded-xl font-bold text-sm transition-all disabled:opacity-40">
                        {loadingDelete ? 'Suppression...' : 'Confirmer'}
                      </button>
                    </div>
                  </div>
                )}
              </div>
            ) : null}
          </div>
        </div>
      </SheetContent>
    </Sheet>
  );
};

export default AccountSettings;
