// List of ALL localStorage keys used for user data
const USER_DATA_KEYS = [
  'brozr_token',
  'brozr_user',
  'brozr_match_filters',
  // Don't clear remember me credentials - they are for convenience
  // 'brozr_saved_email',
  // 'brozr_saved_password',
  // 'brozr_remember_me',
];

export const setAuthToken = (token) => {
  localStorage.setItem('brozr_token', token);
};

export const getAuthToken = () => {
  return localStorage.getItem('brozr_token');
};

export const removeAuthToken = () => {
  localStorage.removeItem('brozr_token');
};

/**
 * CRITICAL: Clear ALL user-specific data from localStorage
 * This MUST be called on logout AND on new login to prevent data leakage between accounts
 */
export const clearAllUserData = () => {
  USER_DATA_KEYS.forEach(key => {
    localStorage.removeItem(key);
  });
};

export const isAuthenticated = () => {
  return !!getAuthToken();
};

export const decodeToken = (token) => {
  try {
    const base64Url = token.split('.')[1];
    const base64 = base64Url.replace(/-/g, '+').replace(/_/g, '/');
    const jsonPayload = decodeURIComponent(
      atob(base64)
        .split('')
        .map((c) => '%' + ('00' + c.charCodeAt(0).toString(16)).slice(-2))
        .join('')
    );
    return JSON.parse(jsonPayload);
  } catch (error) {
    return null;
  }
};

export const getCurrentUser = () => {
  const token = getAuthToken();
  if (!token) return null;
  return decodeToken(token);
};
