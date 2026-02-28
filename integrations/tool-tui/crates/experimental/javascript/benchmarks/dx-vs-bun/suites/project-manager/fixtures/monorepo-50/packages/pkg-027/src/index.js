/**
 * Package: pkg-027
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-026';
// import { something } from 'pkg-025';
// import { something } from 'pkg-024';

export function main() {
    console.log('pkg-027 main function');
    return { name: 'pkg-027', version: '1.0.0' };
}

export function helper27() {
    return 27 * 2;
}

export const config = {
    packageNumber: 27,
    totalPackages: 50
};
