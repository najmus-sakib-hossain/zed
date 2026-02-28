/**
 * Package: pkg-008
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-007';
// import { something } from 'pkg-006';
// import { something } from 'pkg-005';

export function main() {
    console.log('pkg-008 main function');
    return { name: 'pkg-008', version: '1.0.0' };
}

export function helper8() {
    return 8 * 2;
}

export const config = {
    packageNumber: 8,
    totalPackages: 10
};
