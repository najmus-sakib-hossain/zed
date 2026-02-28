/**
 * Package: pkg-012
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-011';
// import { something } from 'pkg-010';
// import { something } from 'pkg-009';

export function main() {
    console.log('pkg-012 main function');
    return { name: 'pkg-012', version: '1.0.0' };
}

export function helper12() {
    return 12 * 2;
}

export const config = {
    packageNumber: 12,
    totalPackages: 100
};
