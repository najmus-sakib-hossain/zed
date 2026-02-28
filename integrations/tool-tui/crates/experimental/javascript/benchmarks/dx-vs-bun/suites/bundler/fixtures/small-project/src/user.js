// User utilities
export function createUser(name, email) {
    return {
        id: Math.random().toString(36).substr(2, 9),
        name,
        email,
        createdAt: new Date().toISOString()
    };
}

export function validateUser(user) {
    return user && user.name && user.email && user.email.includes('@');
}

export function formatUser(user) {
    return `${user.name} <${user.email}>`;
}
