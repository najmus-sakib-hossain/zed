/**
 * Package: pkg-088
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-087';
// import { something } from 'pkg-086';
// import { something } from 'pkg-085';

export function main() {
    console.log('pkg-088 main function');
    return { name: 'pkg-088', version: '1.0.0' };
}

export function helper88() {
    return 88 * 2;
}

export const config = {
    packageNumber: 88,
    totalPackages: 100
};
