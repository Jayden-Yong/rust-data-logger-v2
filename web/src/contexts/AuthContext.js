import React, { createContext, useContext, useState, useEffect } from 'react';
import axios from 'axios';

const AuthContext = createContext();

export const useAuth = () => {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
};

export const AuthProvider = ({ children }) => {
  const [user, setUser] = useState(null);
  const [loading, setLoading] = useState(true);

  // Check for existing session on app start
  useEffect(() => {
    const checkSession = async () => {
      const token = localStorage.getItem('session_token');
      const userData = localStorage.getItem('user');
      const expiresAt = localStorage.getItem('expires_at');

      if (token && userData && expiresAt) {
        // Check if session is still valid
        const now = new Date();
        const expiration = new Date(expiresAt);

        if (now < expiration) {
          // Set axios default header
          axios.defaults.headers.common['Authorization'] = `Bearer ${token}`;
          
          try {
            // Verify session with server
            const response = await axios.get('/api/verify-session');
            if (response.data.success) {
              setUser(JSON.parse(userData));
            } else {
              // Session invalid, clear storage
              clearSession();
            }
          } catch (error) {
            // Session verification failed, clear storage
            clearSession();
          }
        } else {
          // Session expired, clear storage
          clearSession();
        }
      }
      setLoading(false);
    };

    checkSession();
  }, []);

  const clearSession = () => {
    localStorage.removeItem('session_token');
    localStorage.removeItem('user');
    localStorage.removeItem('expires_at');
    delete axios.defaults.headers.common['Authorization'];
    setUser(null);
  };

  const login = (userData) => {
    setUser(userData);
  };

  const logout = async () => {
    try {
      await axios.post('/api/logout');
    } catch (error) {
      console.error('Logout error:', error);
    } finally {
      clearSession();
    }
  };

  const value = {
    user,
    login,
    logout,
    loading,
    isAuthenticated: !!user,
    isAdmin: user?.role === 'admin',
    isInstaller: user?.role === 'installer',
  };

  return (
    <AuthContext.Provider value={value}>
      {children}
    </AuthContext.Provider>
  );
};

export default AuthContext;