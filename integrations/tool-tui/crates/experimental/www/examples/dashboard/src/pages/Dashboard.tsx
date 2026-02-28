import { useState, useEffect, useContext } from 'dx';
import { Sidebar } from '../components/Sidebar';
import { AuthContext } from '../components/AuthProvider';

interface ChartData {
    label: string;
    value: number;
}

interface ApiStats {
    totalUsers: number;
    activeSessions: number;
    revenue: number;
    conversion: number;
    chartData: ChartData[];
}

// Simulated API fetch - demonstrates data fetching pattern
async function fetchDashboardStats(): Promise<ApiStats> {
    // Simulate network delay
    await new Promise(resolve => setTimeout(resolve, 300));

    return {
        totalUsers: 1234,
        activeSessions: 567,
        revenue: 12345,
        conversion: 3.2,
        chartData: [
            { label: 'Mon', value: 120 },
            { label: 'Tue', value: 150 },
            { label: 'Wed', value: 180 },
            { label: 'Thu', value: 140 },
            { label: 'Fri', value: 200 },
            { label: 'Sat', value: 90 },
            { label: 'Sun', value: 75 },
        ],
    };
}

// Simple bar chart component - demonstrates visualization
function BarChart({ data, height = 150 }: { data: ChartData[]; height?: number }) {
    const maxValue = Math.max(...data.map(d => d.value));

    return (
        <div class="bar-chart" style={{ height: `${height}px` }}>
            <div class="chart-bars">
                {data.map(item => {
                    const barHeight = (item.value / maxValue) * 100;
                    return (
                        <div key={item.label} class="bar-container">
                            <div
                                class="bar"
                                style={{ height: `${barHeight}%` }}
                                title={`${item.label}: ${item.value}`}
                            >
                                <span class="bar-value">{item.value}</span>
                            </div>
                            <span class="bar-label">{item.label}</span>
                        </div>
                    );
                })}
            </div>
        </div>
    );
}

export function Dashboard() {
    const auth = useContext(AuthContext);
    const [stats, setStats] = useState<ApiStats | null>(null);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    // Data fetching with useEffect - demonstrates API integration pattern
    useEffect(() => {
        let cancelled = false;

        async function loadStats() {
            try {
                setIsLoading(true);
                setError(null);
                const data = await fetchDashboardStats();
                if (!cancelled) {
                    setStats(data);
                }
            } catch (err) {
                if (!cancelled) {
                    setError('Failed to load dashboard data');
                }
            } finally {
                if (!cancelled) {
                    setIsLoading(false);
                }
            }
        }

        loadStats();

        // Cleanup function to prevent state updates on unmounted component
        return () => {
            cancelled = true;
        };
    }, []);

    const statCards = stats ? [
        { label: 'Total Users', value: stats.totalUsers.toLocaleString(), change: '+12%' },
        { label: 'Active Sessions', value: stats.activeSessions.toLocaleString(), change: '+5%' },
        { label: 'Revenue', value: `$${stats.revenue.toLocaleString()}`, change: '+8%' },
        { label: 'Conversion', value: `${stats.conversion}%`, change: '-2%' },
    ] : [];

    return (
        <div class="dashboard-layout">
            <Sidebar />
            <main class="dashboard-main">
                <header class="dashboard-header">
                    <h1>Dashboard</h1>
                    <p>Welcome back, {auth?.user?.name}!</p>
                </header>

                {isLoading && (
                    <div class="loading-state">
                        <p>Loading dashboard data...</p>
                    </div>
                )}

                {error && (
                    <div class="error-state" role="alert">
                        <p>{error}</p>
                        <button onClick={() => window.location.reload()}>Retry</button>
                    </div>
                )}

                {stats && (
                    <>
                        <section class="stats-grid">
                            {statCards.map(stat => (
                                <div key={stat.label} class="stat-card">
                                    <h3>{stat.label}</h3>
                                    <p class="value">{stat.value}</p>
                                    <span class={`change ${stat.change.startsWith('+') ? 'positive' : 'negative'}`}>
                                        {stat.change}
                                    </span>
                                </div>
                            ))}
                        </section>

                        <section class="chart-section">
                            <h2>Weekly Activity</h2>
                            <p class="chart-description">User activity over the past 7 days</p>
                            <BarChart data={stats.chartData} height={180} />
                        </section>

                        <section class="recent-activity">
                            <h2>Recent Activity</h2>
                            <ul class="activity-list">
                                <li>
                                    <span class="time">2 min ago</span>
                                    <span class="event">New user registered</span>
                                </li>
                                <li>
                                    <span class="time">15 min ago</span>
                                    <span class="event">Order #1234 completed</span>
                                </li>
                                <li>
                                    <span class="time">1 hour ago</span>
                                    <span class="event">System backup completed</span>
                                </li>
                                <li>
                                    <span class="time">3 hours ago</span>
                                    <span class="event">New feature deployed</span>
                                </li>
                            </ul>
                        </section>
                    </>
                )}
            </main>
        </div>
    );
}
