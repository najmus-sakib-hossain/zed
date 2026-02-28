/**
 * Package: pkg-025
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-024';
// import { something } from 'pkg-023';
// import { something } from 'pkg-022';

export function main() {
    console.log('pkg-025 main function');
    return { name: 'pkg-025', version: '1.0.0' };
}

export function helper25() {
    return 25 * 2;
}

export const config = {
    packageNumber: 25,
    totalPackages: 50
};
