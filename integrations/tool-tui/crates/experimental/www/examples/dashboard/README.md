
# Dashboard Example

A dashboard application demonstrating authentication, guards, data fetching, and visualization with dx-www.

## Features

- User authentication with Ed25519 tokens
- Protected routes with guards
- Role-based access control
- Session management
- Data fetching patterns with loading/error states
- Simple chart visualization component

## Running

```bash
cd examples/dashboard dx dev ```
Open //localhost:3000 to see the app.


## Test Credentials


- Admin: admin@example.com / admin123
- User: user@example.com / user123


## Building


```bash
dx build ```

## Key Patterns Demonstrated

### Data Fetching

The Dashboard page demonstrates the recommended data fetching pattern:
```tsx
useEffect(() => { let cancelled = false;
async function loadData() { try { setIsLoading(true);
const data = await fetchFromApi();
if (!cancelled) setData(data);
} catch (err) { if (!cancelled) setError('Failed to load');
} finally { if (!cancelled) setIsLoading(false);
}
}
loadData();
return () => { cancelled = true; };
}, []);
```

### Protected Routes

Routes can require authentication and specific roles:
```tsx
<ProtectedRoute path="/admin" component={Admin} requiredRole="admin" /> ```


### Chart Visualization


Simple bar chart component showing how to build visualizations:
```tsx
<BarChart data={chartData} height={180} /> ```

## Project Structure

@tree:dashboard[]
