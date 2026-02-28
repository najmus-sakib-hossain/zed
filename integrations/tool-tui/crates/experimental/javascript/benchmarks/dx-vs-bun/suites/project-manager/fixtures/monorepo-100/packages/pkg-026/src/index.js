/**
 * Package: pkg-026
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-025';
// import { something } from 'pkg-024';
// import { something } from 'pkg-023';

export function main() {
    console.log('pkg-026 main function');
    return { name: 'pkg-026', version: '1.0.0' };
}

export function helper26() {
    return 26 * 2;
}

export const config = {
    packageNumber: 26,
    totalPackages: 100
};
