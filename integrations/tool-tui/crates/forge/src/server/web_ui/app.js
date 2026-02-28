// DX Forge Web UI Application

// State management
const state = {
    token: "guest",
    user: { username: "guest", role: "admin" },
    ws: null,
    currentPath: '',
    currentFile: null,
    operations: [],
    users: []
};

// Initialize application
// Initialize application
document.addEventListener('DOMContentLoaded', () => {
    // Bypass login
    state.token = "guest";
    initApp();
});

// Login functionality
function showLoginPage() {
    document.getElementById('login-page').style.display = 'flex';
    document.getElementById('app').style.display = 'none';

    document.getElementById('login-form').addEventListener('submit', async (e) => {
        e.preventDefault();
        const username = document.getElementById('username').value;
        const password = document.getElementById('password').value;

        try {
            const response = await fetch('/api/v1/auth/login', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ username, password })
            });

            if (response.ok) {
                const data = await response.json();
                state.token = data.token;
                localStorage.setItem('forge_token', data.token);
                initApp();
            } else {
                showError('Invalid username or password');
            }
        } catch (error) {
            showError('Connection error');
        }
    });
}

function showError(message) {
    const errorEl = document.getElementById('login-error');
    errorEl.textContent = message;
    errorEl.classList.add('show');
    setTimeout(() => errorEl.classList.remove('show'), 3000);
}

// Initialize main application
async function validateTokenAndInit() {
    try {
        const response = await fetch('/api/v1/auth/validate', {
            headers: { 'Authorization': `Bearer ${state.token}` }
        });

        if (response.ok) {
            initApp();
        } else {
            localStorage.removeItem('forge_token');
            state.token = null;
            showLoginPage();
        }
    } catch (error) {
        showLoginPage();
    }
}

async function initApp() {
    document.getElementById('login-page').style.display = 'none';
    document.getElementById('app').style.display = 'flex';

    // Fetch user info
    await fetchUserInfo();

    // Initialize tabs
    initTabs();

    // Load initial data
    loadRepositoryBrowser();
    loadTimeline();

    // Connect WebSocket
    connectWebSocket();

    // Setup logout
    document.getElementById('logout-btn').addEventListener('click', logout);

    // Setup user management
    setupUserManagement();

    // Setup settings
    setupSettings();
}

async function fetchUserInfo() {
    try {
        const response = await fetch('/api/v1/auth/me', {
            headers: { 'Authorization': `Bearer ${state.token}` }
        });

        if (response.ok) {
            state.user = await response.json();
            document.getElementById('current-user').textContent = state.user.username;
            document.getElementById('user-role').textContent = state.user.role;
        }
    } catch (error) {
        console.error('Failed to fetch user info:', error);
    }
}

function logout() {
    localStorage.removeItem('forge_token');
    state.token = null;
    if (state.ws) {
        state.ws.close();
    }
    showLoginPage();
}

// Tab navigation
function initTabs() {
    const navItems = document.querySelectorAll('.nav-item');
    navItems.forEach(item => {
        item.addEventListener('click', () => {
            const tab = item.dataset.tab;
            switchTab(tab);
        });
    });
}

function switchTab(tabName) {
    // Update nav items
    document.querySelectorAll('.nav-item').forEach(item => {
        item.classList.toggle('active', item.dataset.tab === tabName);
    });

    // Update tab content
    document.querySelectorAll('.tab-content').forEach(content => {
        content.classList.toggle('active', content.id === `${tabName}-tab`);
    });

    // Load data for the tab
    if (tabName === 'users') {
        loadUsers();
    }
}

// Repository Browser
async function loadRepositoryBrowser() {
    const treeEl = document.getElementById('file-tree');
    treeEl.innerHTML = '<div class="loading">Loading files...</div>';

    try {
        const response = await fetch('/api/v1/files', {
            headers: { 'Authorization': `Bearer ${state.token}` }
        });

        if (response.ok) {
            const files = await response.json();
            renderFileTree(files);
        }
    } catch (error) {
        treeEl.innerHTML = '<div class="error">Failed to load files</div>';
    }
}

function renderFileTree(files) {
    const treeEl = document.getElementById('file-tree');
    treeEl.innerHTML = '';

    files.forEach(file => {
        const item = document.createElement('div');
        item.className = 'file-item';
        item.innerHTML = `
            <span>${file.is_dir ? '📁' : '📄'}</span>
            <span>${file.name}</span>
        `;

        if (!file.is_dir) {
            item.addEventListener('click', (e) => loadFile(file.path, e.currentTarget));
        }

        treeEl.appendChild(item);
    });
}

async function loadFile(path, element) {
    const contentEl = document.getElementById('file-content');
    const nameEl = document.getElementById('file-name');

    contentEl.innerHTML = '<div class="loading">Loading file...</div>';
    // Handle both forward and backward slashes for display
    nameEl.textContent = path.split(/[/\\]/).pop();

    try {
        const response = await fetch(`/api/v1/files/${encodeURIComponent(path)}`, {
            headers: { 'Authorization': `Bearer ${state.token}` }
        });

        if (response.ok) {
            const data = await response.json();
            renderFileContent(data.content, path);
            state.currentFile = path;

            // Update selected state
            document.querySelectorAll('.file-item').forEach(item => {
                item.classList.remove('selected');
            });
            if (element) {
                element.classList.add('selected');
            }
        } else {
            contentEl.innerHTML = `<div class="error">Failed to load file: Server returned ${response.status}</div>`;
        }
    } catch (error) {
        contentEl.innerHTML = '<div class="error">Failed to load file</div>';
    }
}

function renderFileContent(content, path) {
    const contentEl = document.getElementById('file-content');
    const ext = path.split('.').pop();
    const language = getLanguage(ext);

    contentEl.innerHTML = `
        <pre><code class="language-${language}">${escapeHtml(content)}</code></pre>
    `;

    // Apply syntax highlighting if available
    if (window.Prism) {
        Prism.highlightAll();
    }
}

function getLanguage(ext) {
    const languageMap = {
        'rs': 'rust',
        'js': 'javascript',
        'ts': 'typescript',
        'py': 'python',
        'html': 'html',
        'css': 'css',
        'json': 'json',
        'md': 'markdown',
        'toml': 'toml',
        'yaml': 'yaml',
        'yml': 'yaml'
    };
    return languageMap[ext] || 'plaintext';
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// Copy code functionality
document.getElementById('copy-code-btn').addEventListener('click', () => {
    const code = document.querySelector('#file-content code');
    if (code) {
        navigator.clipboard.writeText(code.textContent);
        showNotification('Code copied to clipboard!');
    }
});

// Timeline
async function loadTimeline() {
    const timelineEl = document.getElementById('timeline-content');
    timelineEl.innerHTML = '<div class="loading">Loading operations...</div>';

    try {
        const response = await fetch('/ops?limit=50', {
            headers: { 'Authorization': `Bearer ${state.token}` }
        });

        if (response.ok) {
            const operations = await response.json();
            state.operations = operations;
            renderTimeline(operations);
        }
    } catch (error) {
        timelineEl.innerHTML = '<div class="error">Failed to load timeline</div>';
    }
}

function renderTimeline(operations) {
    const timelineEl = document.getElementById('timeline-content');

    if (operations.length === 0) {
        timelineEl.innerHTML = '<div class="empty-state"><p>No operations yet</p></div>';
        return;
    }

    timelineEl.innerHTML = operations.map(op => `
        <div class="operation-item">
            <div style="display: flex; justify-content: space-between; margin-bottom: 0.5rem;">
                <strong>${getOperationType(op)}</strong>
                <span style="color: var(--text-muted); font-size: 0.875rem;">${formatTimestamp(op.timestamp)}</span>
            </div>
            <div style="color: var(--text-secondary); font-size: 0.875rem;">
                ${op.file || 'Unknown file'}
            </div>
            <div style="color: var(--text-muted); font-size: 0.813rem; margin-top: 0.25rem;">
                Actor: ${op.actor_id || 'Unknown'}
            </div>
        </div>
    `).join('');
}

function getOperationType(op) {
    if (op.Insert) return '✨ Insert';
    if (op.Delete) return '🗑️ Delete';
    if (op.Update) return '✏️ Update';
    return '❓ Unknown';
}

function formatTimestamp(timestamp) {
    if (!timestamp) return 'Unknown time';
    const date = new Date(timestamp * 1000);
    return date.toLocaleString();
}

// Timeline filtering
document.getElementById('timeline-search').addEventListener('input', (e) => {
    const query = e.target.value.toLowerCase();
    filterTimeline(query);
});

document.getElementById('timeline-filter').addEventListener('change', (e) => {
    const filter = e.target.value;
    applyFilterType(filter);
});

function filterTimeline(query) {
    const filtered = state.operations.filter(op => {
        const file = op.file || '';
        const actor = op.actor_id || '';
        return file.toLowerCase().includes(query) || actor.toLowerCase().includes(query);
    });
    renderTimeline(filtered);
}

function applyFilterType(type) {
    if (type === 'all') {
        renderTimeline(state.operations);
        return;
    }

    const filtered = state.operations.filter(op => {
        return op[type.charAt(0).toUpperCase() + type.slice(1)] !== undefined;
    });
    renderTimeline(filtered);
}

// WebSocket Connection
function connectWebSocket() {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const ws = new WebSocket(`${protocol}//${window.location.host}/ws`);

    ws.onopen = () => {
        console.log('WebSocket connected');
    };

    ws.onmessage = (event) => {
        try {
            const message = JSON.parse(event.data);
            if (message.Operation) {
                handleNewOperation(message.Operation.operation);
            }
        } catch (error) {
            console.error('WebSocket message error:', error);
        }
    };

    ws.onerror = (error) => {
        console.error('WebSocket error:', error);
    };

    ws.onclose = () => {
        console.log('WebSocket disconnected, reconnecting...');
        setTimeout(connectWebSocket, 3000);
    };

    state.ws = ws;
}

function handleNewOperation(operation) {
    state.operations.unshift(operation);
    if (state.operations.length > 100) {
        state.operations.pop();
    }

    // Update timeline if visible
    const timelineTab = document.getElementById('timeline-tab');
    if (timelineTab.classList.contains('active')) {
        renderTimeline(state.operations);
    }
}

// User Management
function setupUserManagement() {
    const addUserBtn = document.getElementById('add-user-btn');
    const modal = document.getElementById('add-user-modal');
    const closeBtn = modal.querySelector('.modal-close');
    const cancelBtn = modal.querySelector('.modal-cancel');
    const form = document.getElementById('add-user-form');

    addUserBtn.addEventListener('click', () => {
        modal.classList.add('show');
    });

    closeBtn.addEventListener('click', () => {
        modal.classList.remove('show');
    });

    cancelBtn.addEventListener('click', () => {
        modal.classList.remove('show');
    });

    form.addEventListener('submit', async (e) => {
        e.preventDefault();
        const formData = new FormData(form);
        const userData = {
            username: formData.get('username'),
            password: formData.get('password'),
            email: formData.get('email'),
            role: formData.get('role')
        };

        try {
            const response = await fetch('/api/v1/users', {
                method: 'POST',
                headers: {
                    'Authorization': `Bearer ${state.token}`,
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify(userData)
            });

            if (response.ok) {
                showNotification('User created successfully');
                modal.classList.remove('show');
                form.reset();
                loadUsers();
            } else {
                const error = await response.json();
                showNotification(error.error || 'Failed to create user', 'error');
            }
        } catch (error) {
            showNotification('Connection error', 'error');
        }
    });
}

async function loadUsers() {
    const usersEl = document.getElementById('users-list');
    usersEl.innerHTML = '<div class="loading">Loading users...</div>';

    try {
        const response = await fetch('/api/v1/users', {
            headers: { 'Authorization': `Bearer ${state.token}` }
        });

        if (response.ok) {
            const users = await response.json();
            state.users = users;
            renderUsers(users);
        }
    } catch (error) {
        usersEl.innerHTML = '<div class="error">Failed to load users</div>';
    }
}

function renderUsers(users) {
    const usersEl = document.getElementById('users-list');

    usersEl.innerHTML = users.map(user => `
        <div class="user-card">
            <div>
                <div style="font-weight: 600;">${user.username}</div>
                <div style="font-size: 0.875rem; color: var(--text-secondary); margin-top: 0.25rem;">
                    ${user.email || 'No email'} • ${user.role}
                </div>
            </div>
            <div>
                ${state.user.role === 'admin' && user.username !== state.user.username ?
            `<button class="btn-icon" onclick="deleteUser('${user.username}')">🗑️</button>` : ''
        }
            </div>
        </div>
    `).join('');
}

async function deleteUser(username) {
    if (!confirm(`Are you sure you want to delete user "${username}"?`)) {
        return;
    }

    try {
        const response = await fetch(`/api/v1/users/${username}`, {
            method: 'DELETE',
            headers: { 'Authorization': `Bearer ${state.token}` }
        });

        if (response.ok) {
            showNotification('User deleted successfully');
            loadUsers();
        } else {
            showNotification('Failed to delete user', 'error');
        }
    } catch (error) {
        showNotification('Connection error', 'error');
    }
}

// Settings
function setupSettings() {
    // Auto-update toggle
    const autoUpdateToggle = document.getElementById('auto-update-enabled');
    autoUpdateToggle.checked = localStorage.getItem('auto_update_enabled') !== 'false';

    autoUpdateToggle.addEventListener('change', (e) => {
        localStorage.setItem('auto_update_enabled', e.target.checked);
        showNotification(`Auto-update ${e.target.checked ? 'enabled' : 'disabled'}`);
    });

    // Dark mode toggle (always on for now)
    const darkModeToggle = document.getElementById('dark-mode-toggle');
    darkModeToggle.checked = true;

    // Password change
    const passwordForm = document.getElementById('change-password-form');
    passwordForm.addEventListener('submit', async (e) => {
        e.preventDefault();

        const currentPassword = document.getElementById('current-password').value;
        const newPassword = document.getElementById('new-password').value;
        const confirmPassword = document.getElementById('confirm-password').value;

        if (newPassword !== confirmPassword) {
            showNotification('Passwords do not match', 'error');
            return;
        }

        try {
            const response = await fetch('/api/v1/auth/change-password', {
                method: 'POST',
                headers: {
                    'Authorization': `Bearer ${state.token}`,
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({
                    old_password: currentPassword,
                    new_password: newPassword
                })
            });

            if (response.ok) {
                showNotification('Password changed successfully');
                passwordForm.reset();
            } else {
                const error = await response.json();
                showNotification(error.error || 'Failed to change password', 'error');
            }
        } catch (error) {
            showNotification('Connection error', 'error');
        }
    });
}

// Notifications
function showNotification(message, type = 'success') {
    // Simple notification - could be enhanced with a toast library
    alert(message);
}

// Fetch repo info
fetch('/health')
    .then(r => r.json())
    .then(data => {
        document.getElementById('repo-name').textContent = 'forge@' + (data.version || '0.0.2');
    })
    .catch(() => {
        document.getElementById('repo-name').textContent = 'Unknown';
    });
