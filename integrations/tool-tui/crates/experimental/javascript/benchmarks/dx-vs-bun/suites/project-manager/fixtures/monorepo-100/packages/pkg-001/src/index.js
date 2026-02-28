/**
 * Package: pkg-001
 * Part of monorepo benchmark fixture
 */



export function main() {
    console.log('pkg-001 main function');
    return { name: 'pkg-001', version: '1.0.0' };
}

export function helper1() {
    return 1 * 2;
}

export const config = {
    packageNumber: 1,
    totalPackages: 100
};
