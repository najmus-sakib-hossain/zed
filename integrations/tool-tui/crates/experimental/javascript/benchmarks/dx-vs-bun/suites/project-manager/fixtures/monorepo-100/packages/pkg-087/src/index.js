/**
 * Package: pkg-087
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-086';
// import { something } from 'pkg-085';
// import { something } from 'pkg-084';

export function main() {
    console.log('pkg-087 main function');
    return { name: 'pkg-087', version: '1.0.0' };
}

export function helper87() {
    return 87 * 2;
}

export const config = {
    packageNumber: 87,
    totalPackages: 100
};
