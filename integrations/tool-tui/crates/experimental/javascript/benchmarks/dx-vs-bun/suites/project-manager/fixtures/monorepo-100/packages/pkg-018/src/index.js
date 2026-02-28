/**
 * Package: pkg-018
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-017';
// import { something } from 'pkg-016';
// import { something } from 'pkg-015';

export function main() {
    console.log('pkg-018 main function');
    return { name: 'pkg-018', version: '1.0.0' };
}

export function helper18() {
    return 18 * 2;
}

export const config = {
    packageNumber: 18,
    totalPackages: 100
};
