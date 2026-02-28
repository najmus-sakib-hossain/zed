import { useState } from 'dx';
import { Sidebar } from '../components/Sidebar';

interface User {
    id: string;
    name: string;
    email: string;
    role: 'user' | 'admin';
    status: 'active' | 'inactive';
}

export function Admin() {
    const [users] = useState<User[]>([
        { id: '1', name: 'Admin User', email: 'admin@example.com', role: 'admin', status: 'active' },
        { id: '2', name: 'Regular User', email: 'user@example.com', role: 'user', status: 'active' },
        { id: '3', name: 'Jane Doe', email: 'jane@example.com', role: 'user', status: 'active' },
        { id: '4', name: 'John Smith', email: 'john@example.com', role: 'user', status: 'inactive' },
    ]);

    return (
        <div class="dashboard-layout">
            <Sidebar />
            <main class="dashboard-main">
                <header class="dashboard-header">
                    <h1>Admin Panel</h1>
                    <p>Manage users and system settings</p>
                </header>

                <section class="admin-section">
                    <h2>User Management</h2>

                    <table class="users-table">
                        <thead>
                            <tr>
                                <th>Name</th>
                                <th>Email</th>
                                <th>Role</th>
                                <th>Status</th>
                                <th>Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            {users.map(user => (
                                <tr key={user.id}>
                                    <td>{user.name}</td>
                                    <td>{user.email}</td>
                                    <td>
                                        <span class={`badge role-${user.role}`}>
                                            {user.role}
                                        </span>
                                    </td>
                                    <td>
                                        <span class={`badge status-${user.status}`}>
                                            {user.status}
                                        </span>
                                    </td>
                                    <td>
                                        <button class="action-btn">Edit</button>
                                        <button class="action-btn danger">Delete</button>
                                    </td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </section>

                <section class="admin-section">
                    <h2>System Status</h2>
                    <div class="system-status">
                        <div class="status-item">
                            <span class="label">Server Status</span>
                            <span class="value online">Online</span>
                        </div>
                        <div class="status-item">
                            <span class="label">Database</span>
                            <span class="value online">Connected</span>
                        </div>
                        <div class="status-item">
                            <span class="label">Cache</span>
                            <span class="value online">Active</span>
                        </div>
                        <div class="status-item">
                            <span class="label">Last Backup</span>
                            <span class="value">2 hours ago</span>
                        </div>
                    </div>
                </section>
            </main>
        </div>
    );
}
