// Large sample JavaScript file for benchmarking
// This file contains various JavaScript patterns for comprehensive testing

import React, { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { useDispatch, useSelector } from 'react-redux';
import axios from 'axios';

// Constants
const API_BASE_URL = 'https://api.example.com';
const MAX_RETRIES = 3;
const TIMEOUT_MS = 5000;

// Types (for TypeScript compatibility testing)
/**
 * @typedef {Object} User
 * @property {number} id
 * @property {string} name
 * @property {string} email
 * @property {string} avatar
 * @property {Date} createdAt
 */

/**
 * @typedef {Object} Post
 * @property {number} id
 * @property {string} title
 * @property {string} content
 * @property {number} authorId
 * @property {string[]} tags
 * @property {Date} publishedAt
 */

// Utility functions
const debounce = (fn, delay) => {
    let timeoutId;
    return (...args) => {
        clearTimeout(timeoutId);
        timeoutId = setTimeout(() => fn(...args), delay);
    };
};

const throttle = (fn, limit) => {
    let inThrottle;
    return (...args) => {
        if (!inThrottle) {
            fn(...args);
            inThrottle = true;
            setTimeout(() => (inThrottle = false), limit);
        }
    };
};

const formatDate = (date) => {
    const options = { year: 'numeric', month: 'long', day: 'numeric' };
    return new Date(date).toLocaleDateString('en-US', options);
};

const truncateText = (text, maxLength) => {
    if (text.length <= maxLength) return text;
    return text.slice(0, maxLength) + '...';
};

// API client
class ApiClient {
    constructor(baseUrl) {
        this.baseUrl = baseUrl;
        this.retryCount = 0;
    }

    async request(endpoint, options = {}) {
        const url = `${this.baseUrl}${endpoint}`;
        const config = {
            timeout: TIMEOUT_MS,
            headers: {
                'Content-Type': 'application/json',
                ...options.headers,
            },
            ...options,
        };

        try {
            const response = await axios(url, config);
            this.retryCount = 0;
            return response.data;
        } catch (error) {
            if (this.retryCount < MAX_RETRIES) {
                this.retryCount++;
                return this.request(endpoint, options);
            }
            throw error;
        }
    }

    async get(endpoint) {
        return this.request(endpoint, { method: 'GET' });
    }

    async post(endpoint, data) {
        return this.request(endpoint, { method: 'POST', data });
    }

    async put(endpoint, data) {
        return this.request(endpoint, { method: 'PUT', data });
    }

    async delete(endpoint) {
        return this.request(endpoint, { method: 'DELETE' });
    }
}

const apiClient = new ApiClient(API_BASE_URL);

// Custom hooks
const useLocalStorage = (key, initialValue) => {
    const [storedValue, setStoredValue] = useState(() => {
        try {
            const item = window.localStorage.getItem(key);
            return item ? JSON.parse(item) : initialValue;
        } catch (error) {
            return initialValue;
        }
    });

    const setValue = (value) => {
        try {
            const valueToStore = value instanceof Function ? value(storedValue) : value;
            setStoredValue(valueToStore);
            window.localStorage.setItem(key, JSON.stringify(valueToStore));
        } catch (error) {
            // Handle error
        }
    };

    return [storedValue, setValue];
};

const useDebounce = (value, delay) => {
    const [debouncedValue, setDebouncedValue] = useState(value);

    useEffect(() => {
        const handler = setTimeout(() => {
            setDebouncedValue(value);
        }, delay);

        return () => {
            clearTimeout(handler);
        };
    }, [value, delay]);

    return debouncedValue;
};

const usePrevious = (value) => {
    const ref = useRef();
    useEffect(() => {
        ref.current = value;
    });
    return ref.current;
};

const useOnClickOutside = (ref, handler) => {
    useEffect(() => {
        const listener = (event) => {
            if (!ref.current || ref.current.contains(event.target)) {
                return;
            }
            handler(event);
        };

        document.addEventListener('mousedown', listener);
        document.addEventListener('touchstart', listener);

        return () => {
            document.removeEventListener('mousedown', listener);
            document.removeEventListener('touchstart', listener);
        };
    }, [ref, handler]);
};

// Components
const LoadingSpinner = ({ size = 'medium' }) => {
    const sizeClasses = {
        small: 'w-4 h-4',
        medium: 'w-8 h-8',
        large: 'w-12 h-12',
    };

    return (
        <div className={`animate-spin rounded-full border-2 border-gray-300 border-t-blue-600 ${sizeClasses[size]}`} />
    );
};

const ErrorMessage = ({ message, onRetry }) => {
    return (
        <div className="bg-red-50 border border-red-200 rounded-lg p-4">
            <div className="flex items-center">
                <svg className="w-5 h-5 text-red-500 mr-2" fill="currentColor" viewBox="0 0 20 20">
                    <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clipRule="evenodd" />
                </svg>
                <span className="text-red-700">{message}</span>
            </div>
            {onRetry && (
                <button
                    onClick={onRetry}
                    className="mt-2 text-sm text-red-600 hover:text-red-800 underline"
                >
                    Try again
                </button>
            )}
        </div>
    );
};

const Avatar = ({ src, alt, size = 'medium' }) => {
    const sizeClasses = {
        small: 'w-8 h-8',
        medium: 'w-12 h-12',
        large: 'w-16 h-16',
    };

    return (
        <img
            src={src || '/default-avatar.png'}
            alt={alt}
            className={`rounded-full object-cover ${sizeClasses[size]}`}
        />
    );
};

const Button = ({ children, variant = 'primary', size = 'medium', disabled, onClick, ...props }) => {
    const baseClasses = 'font-medium rounded-lg transition-colors focus:outline-none focus:ring-2 focus:ring-offset-2';

    const variantClasses = {
        primary: 'bg-blue-600 text-white hover:bg-blue-700 focus:ring-blue-500',
        secondary: 'bg-gray-200 text-gray-800 hover:bg-gray-300 focus:ring-gray-500',
        danger: 'bg-red-600 text-white hover:bg-red-700 focus:ring-red-500',
    };

    const sizeClasses = {
        small: 'px-3 py-1.5 text-sm',
        medium: 'px-4 py-2 text-base',
        large: 'px-6 py-3 text-lg',
    };

    return (
        <button
            className={`${baseClasses} ${variantClasses[variant]} ${sizeClasses[size]} ${disabled ? 'opacity-50 cursor-not-allowed' : ''}`}
            disabled={disabled}
            onClick={onClick}
            {...props}
        >
            {children}
        </button>
    );
};

const Card = ({ title, children, footer }) => {
    return (
        <div className="bg-white rounded-lg shadow-md overflow-hidden">
            {title && (
                <div className="px-6 py-4 border-b border-gray-200">
                    <h3 className="text-lg font-semibold text-gray-900">{title}</h3>
                </div>
            )}
            <div className="px-6 py-4">{children}</div>
            {footer && (
                <div className="px-6 py-4 bg-gray-50 border-t border-gray-200">
                    {footer}
                </div>
            )}
        </div>
    );
};

const Modal = ({ isOpen, onClose, title, children }) => {
    const modalRef = useRef();
    useOnClickOutside(modalRef, onClose);

    useEffect(() => {
        const handleEscape = (event) => {
            if (event.key === 'Escape') {
                onClose();
            }
        };

        if (isOpen) {
            document.addEventListener('keydown', handleEscape);
            document.body.style.overflow = 'hidden';
        }

        return () => {
            document.removeEventListener('keydown', handleEscape);
            document.body.style.overflow = 'unset';
        };
    }, [isOpen, onClose]);

    if (!isOpen) return null;

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
            <div className="fixed inset-0 bg-black bg-opacity-50" />
            <div ref={modalRef} className="relative bg-white rounded-lg shadow-xl max-w-md w-full mx-4">
                <div className="flex items-center justify-between px-6 py-4 border-b">
                    <h2 className="text-xl font-semibold">{title}</h2>
                    <button onClick={onClose} className="text-gray-400 hover:text-gray-600">
                        <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                        </svg>
                    </button>
                </div>
                <div className="px-6 py-4">{children}</div>
            </div>
        </div>
    );
};

// Main application component
const UserDashboard = () => {
    const [users, setUsers] = useState([]);
    const [posts, setPosts] = useState([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState(null);
    const [searchTerm, setSearchTerm] = useState('');
    const [selectedUser, setSelectedUser] = useState(null);
    const [isModalOpen, setIsModalOpen] = useState(false);

    const debouncedSearchTerm = useDebounce(searchTerm, 300);

    const fetchData = useCallback(async () => {
        setLoading(true);
        setError(null);

        try {
            const [usersData, postsData] = await Promise.all([
                apiClient.get('/users'),
                apiClient.get('/posts'),
            ]);
            setUsers(usersData);
            setPosts(postsData);
        } catch (err) {
            setError('Failed to fetch data. Please try again.');
        } finally {
            setLoading(false);
        }
    }, []);

    useEffect(() => {
        fetchData();
    }, [fetchData]);

    const filteredUsers = useMemo(() => {
        if (!debouncedSearchTerm) return users;
        const term = debouncedSearchTerm.toLowerCase();
        return users.filter(
            (user) =>
                user.name.toLowerCase().includes(term) ||
                user.email.toLowerCase().includes(term)
        );
    }, [users, debouncedSearchTerm]);

    const getUserPosts = useCallback(
        (userId) => posts.filter((post) => post.authorId === userId),
        [posts]
    );

    const handleUserClick = (user) => {
        setSelectedUser(user);
        setIsModalOpen(true);
    };

    const handleCloseModal = () => {
        setIsModalOpen(false);
        setSelectedUser(null);
    };

    if (loading) {
        return (
            <div className="flex items-center justify-center min-h-screen">
                <LoadingSpinner size="large" />
            </div>
        );
    }

    if (error) {
        return (
            <div className="flex items-center justify-center min-h-screen p-4">
                <ErrorMessage message={error} onRetry={fetchData} />
            </div>
        );
    }

    return (
        <div className="min-h-screen bg-gray-100 py-8">
            <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
                <div className="mb-8">
                    <h1 className="text-3xl font-bold text-gray-900">User Dashboard</h1>
                    <p className="mt-2 text-gray-600">
                        Manage users and their posts
                    </p>
                </div>

                <div className="mb-6">
                    <input
                        type="text"
                        placeholder="Search users..."
                        value={searchTerm}
                        onChange={(e) => setSearchTerm(e.target.value)}
                        className="w-full max-w-md px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                    />
                </div>

                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                    {filteredUsers.map((user) => (
                        <Card key={user.id}>
                            <div className="flex items-center space-x-4">
                                <Avatar src={user.avatar} alt={user.name} />
                                <div>
                                    <h3 className="font-semibold text-gray-900">{user.name}</h3>
                                    <p className="text-sm text-gray-500">{user.email}</p>
                                </div>
                            </div>
                            <div className="mt-4">
                                <p className="text-sm text-gray-600">
                                    {getUserPosts(user.id).length} posts
                                </p>
                                <p className="text-sm text-gray-500">
                                    Joined {formatDate(user.createdAt)}
                                </p>
                            </div>
                            <div className="mt-4">
                                <Button
                                    variant="secondary"
                                    size="small"
                                    onClick={() => handleUserClick(user)}
                                >
                                    View Details
                                </Button>
                            </div>
                        </Card>
                    ))}
                </div>

                {filteredUsers.length === 0 && (
                    <div className="text-center py-12">
                        <p className="text-gray-500">No users found matching your search.</p>
                    </div>
                )}

                <Modal
                    isOpen={isModalOpen}
                    onClose={handleCloseModal}
                    title={selectedUser?.name || 'User Details'}
                >
                    {selectedUser && (
                        <div>
                            <div className="flex items-center space-x-4 mb-4">
                                <Avatar src={selectedUser.avatar} alt={selectedUser.name} size="large" />
                                <div>
                                    <h3 className="text-xl font-semibold">{selectedUser.name}</h3>
                                    <p className="text-gray-500">{selectedUser.email}</p>
                                </div>
                            </div>
                            <div className="border-t pt-4">
                                <h4 className="font-semibold mb-2">Recent Posts</h4>
                                {getUserPosts(selectedUser.id).slice(0, 3).map((post) => (
                                    <div key={post.id} className="mb-2">
                                        <p className="font-medium">{post.title}</p>
                                        <p className="text-sm text-gray-500">
                                            {truncateText(post.content, 100)}
                                        </p>
                                    </div>
                                ))}
                            </div>
                        </div>
                    )}
                </Modal>
            </div>
        </div>
    );
};

export default UserDashboard;
