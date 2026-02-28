/**
 * Package: pkg-010
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-009';
// import { something } from 'pkg-008';
// import { something } from 'pkg-007';

export function main() {
    console.log('pkg-010 main function');
    return { name: 'pkg-010', version: '1.0.0' };
}

export function helper10() {
    return 10 * 2;
}

export const config = {
    packageNumber: 10,
    totalPackages: 50
};
