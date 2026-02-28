import { useState, useContext } from 'dx';
import { Sidebar } from '../components/Sidebar';
import { AuthContext } from '../components/AuthProvider';

export function Settings() {
    const auth = useContext(AuthContext);
    const [name, setName] = useState(auth?.user?.name || '');
    const [email, setEmail] = useState(auth?.user?.email || '');
    const [notifications, setNotifications] = useState(true);
    const [saved, setSaved] = useState(false);

    const handleSubmit = (e: Event) => {
        e.preventDefault();
        // In a real app, save to server
        setSaved(true);
        setTimeout(() => setSaved(false), 3000);
    };

    return (
        <div class="dashboard-layout">
            <Sidebar />
            <main class="dashboard-main">
                <header class="dashboard-header">
                    <h1>Settings</h1>
                    <p>Manage your account settings</p>
                </header>

                <form class="settings-form" onSubmit={handleSubmit}>
                    {saved && (
                        <div class="success-message" role="alert">
                            Settings saved successfully!
                        </div>
                    )}

                    <section class="settings-section">
                        <h2>Profile</h2>

                        <div class="form-group">
                            <label htmlFor="name">Name</label>
                            <input
                                id="name"
                                type="text"
                                value={name}
                                onChange={(e) => setName((e.target as HTMLInputElement).value)}
                            />
                        </div>

                        <div class="form-group">
                            <label htmlFor="email">Email</label>
                            <input
                                id="email"
                                type="email"
                                value={email}
                                onChange={(e) => setEmail((e.target as HTMLInputElement).value)}
                            />
                        </div>
                    </section>

                    <section class="settings-section">
                        <h2>Preferences</h2>

                        <div class="form-group checkbox">
                            <input
                                id="notifications"
                                type="checkbox"
                                checked={notifications}
                                onChange={(e) => setNotifications((e.target as HTMLInputElement).checked)}
                            />
                            <label htmlFor="notifications">
                                Enable email notifications
                            </label>
                        </div>
                    </section>

                    <button type="submit" class="save-btn">
                        Save Changes
                    </button>
                </form>
            </main>
        </div>
    );
}
