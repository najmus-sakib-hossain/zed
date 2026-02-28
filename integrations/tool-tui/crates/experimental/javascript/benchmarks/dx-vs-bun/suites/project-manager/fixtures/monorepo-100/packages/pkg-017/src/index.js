/**
 * Package: pkg-017
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-016';
// import { something } from 'pkg-015';
// import { something } from 'pkg-014';

export function main() {
    console.log('pkg-017 main function');
    return { name: 'pkg-017', version: '1.0.0' };
}

export function helper17() {
    return 17 * 2;
}

export const config = {
    packageNumber: 17,
    totalPackages: 100
};
