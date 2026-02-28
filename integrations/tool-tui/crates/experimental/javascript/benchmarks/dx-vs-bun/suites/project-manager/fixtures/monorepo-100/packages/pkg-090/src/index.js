/**
 * Package: pkg-090
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-089';
// import { something } from 'pkg-088';
// import { something } from 'pkg-087';

export function main() {
    console.log('pkg-090 main function');
    return { name: 'pkg-090', version: '1.0.0' };
}

export function helper90() {
    return 90 * 2;
}

export const config = {
    packageNumber: 90,
    totalPackages: 100
};
