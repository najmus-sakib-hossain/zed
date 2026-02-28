/**
 * MDN Reference Data for CSS Properties
 * 
 * This module provides CSS property information from MDN (Mozilla Developer Network)
 * for use in hover providers and documentation features.
 * 
 * **Validates: Requirements 2.1, 2.2, 2.4**
 */

/**
 * Browser support information for a CSS property.
 */
export interface BrowserSupport {
    /** Chrome version that first supported this property, or null if unsupported */
    chrome: string | null;
    /** Firefox version that first supported this property, or null if unsupported */
    firefox: string | null;
    /** Safari version that first supported this property, or null if unsupported */
    safari: string | null;
    /** Edge version that first supported this property, or null if unsupported */
    edge: string | null;
    /** Whether this property has limited browser support */
    hasLimitedSupport: boolean;
}

/**
 * MDN property information.
 */
export interface MDNPropertyInfo {
    /** CSS property name (e.g., "display", "padding") */
    propertyName: string;
    /** Human-readable description of the property */
    description: string;
    /** CSS syntax for the property */
    syntax: string;
    /** URL to the full MDN documentation page */
    mdnUrl: string;
    /** Browser support information */
    browserSupport: BrowserSupport;
}

/**
 * MDN reference data for common CSS properties.
 * 
 * This is a subset of CSS properties commonly used with utility-first CSS frameworks.
 */
export const mdnProperties: Map<string, MDNPropertyInfo> = new Map([
    // Display
    ['display', {
        propertyName: 'display',
        description: 'Sets whether an element is treated as a block or inline element and the layout used for its children.',
        syntax: 'block | inline | inline-block | flex | inline-flex | grid | inline-grid | none | contents',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/display',
        browserSupport: { chrome: '1', firefox: '1', safari: '1', edge: '12', hasLimitedSupport: false }
    }],

    // Flexbox
    ['flex', {
        propertyName: 'flex',
        description: 'Shorthand property for flex-grow, flex-shrink, and flex-basis.',
        syntax: 'none | [ <flex-grow> <flex-shrink>? || <flex-basis> ]',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/flex',
        browserSupport: { chrome: '29', firefox: '20', safari: '9', edge: '12', hasLimitedSupport: false }
    }],
    ['flex-direction', {
        propertyName: 'flex-direction',
        description: 'Sets how flex items are placed in the flex container defining the main axis.',
        syntax: 'row | row-reverse | column | column-reverse',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/flex-direction',
        browserSupport: { chrome: '29', firefox: '20', safari: '9', edge: '12', hasLimitedSupport: false }
    }],
    ['justify-content', {
        propertyName: 'justify-content',
        description: 'Defines how the browser distributes space between and around content items along the main axis.',
        syntax: 'flex-start | flex-end | center | space-between | space-around | space-evenly',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/justify-content',
        browserSupport: { chrome: '29', firefox: '20', safari: '9', edge: '12', hasLimitedSupport: false }
    }],
    ['align-items', {
        propertyName: 'align-items',
        description: 'Sets the align-self value on all direct children as a group.',
        syntax: 'stretch | flex-start | flex-end | center | baseline',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/align-items',
        browserSupport: { chrome: '29', firefox: '20', safari: '9', edge: '12', hasLimitedSupport: false }
    }],

    // Grid
    ['grid', {
        propertyName: 'grid',
        description: 'Shorthand property for grid-template-rows, grid-template-columns, grid-template-areas, grid-auto-rows, grid-auto-columns, and grid-auto-flow.',
        syntax: '<grid-template> | <grid-template-rows> / [ auto-flow && dense? ] <grid-auto-columns>?',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/grid',
        browserSupport: { chrome: '57', firefox: '52', safari: '10.1', edge: '16', hasLimitedSupport: false }
    }],
    ['grid-cols', {
        propertyName: 'grid-template-columns',
        description: 'Defines the line names and track sizing functions of the grid columns.',
        syntax: 'none | <track-list> | <auto-track-list>',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/grid-template-columns',
        browserSupport: { chrome: '57', firefox: '52', safari: '10.1', edge: '16', hasLimitedSupport: false }
    }],

    // Spacing
    ['padding', {
        propertyName: 'padding',
        description: 'Shorthand property for padding-top, padding-right, padding-bottom, and padding-left.',
        syntax: '[ <length> | <percentage> ]{1,4}',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/padding',
        browserSupport: { chrome: '1', firefox: '1', safari: '1', edge: '12', hasLimitedSupport: false }
    }],
    ['padding-top', {
        propertyName: 'padding-top',
        description: 'Sets the height of the padding area on the top of an element.',
        syntax: '<length> | <percentage>',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/padding-top',
        browserSupport: { chrome: '1', firefox: '1', safari: '1', edge: '12', hasLimitedSupport: false }
    }],
    ['padding-right', {
        propertyName: 'padding-right',
        description: 'Sets the width of the padding area on the right of an element.',
        syntax: '<length> | <percentage>',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/padding-right',
        browserSupport: { chrome: '1', firefox: '1', safari: '1', edge: '12', hasLimitedSupport: false }
    }],
    ['padding-bottom', {
        propertyName: 'padding-bottom',
        description: 'Sets the height of the padding area on the bottom of an element.',
        syntax: '<length> | <percentage>',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/padding-bottom',
        browserSupport: { chrome: '1', firefox: '1', safari: '1', edge: '12', hasLimitedSupport: false }
    }],
    ['padding-left', {
        propertyName: 'padding-left',
        description: 'Sets the width of the padding area on the left of an element.',
        syntax: '<length> | <percentage>',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/padding-left',
        browserSupport: { chrome: '1', firefox: '1', safari: '1', edge: '12', hasLimitedSupport: false }
    }],
    ['margin', {
        propertyName: 'margin',
        description: 'Shorthand property for margin-top, margin-right, margin-bottom, and margin-left.',
        syntax: '[ <length> | <percentage> | auto ]{1,4}',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/margin',
        browserSupport: { chrome: '1', firefox: '1', safari: '1', edge: '12', hasLimitedSupport: false }
    }],
    ['gap', {
        propertyName: 'gap',
        description: 'Shorthand property for row-gap and column-gap.',
        syntax: '<row-gap> <column-gap>?',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/gap',
        browserSupport: { chrome: '66', firefox: '61', safari: '12', edge: '16', hasLimitedSupport: false }
    }],

    // Sizing
    ['width', {
        propertyName: 'width',
        description: 'Sets an element\'s width.',
        syntax: 'auto | <length> | <percentage> | min-content | max-content | fit-content',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/width',
        browserSupport: { chrome: '1', firefox: '1', safari: '1', edge: '12', hasLimitedSupport: false }
    }],
    ['height', {
        propertyName: 'height',
        description: 'Sets an element\'s height.',
        syntax: 'auto | <length> | <percentage> | min-content | max-content | fit-content',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/height',
        browserSupport: { chrome: '1', firefox: '1', safari: '1', edge: '12', hasLimitedSupport: false }
    }],

    // Typography
    ['font-size', {
        propertyName: 'font-size',
        description: 'Sets the size of the font.',
        syntax: '<absolute-size> | <relative-size> | <length-percentage>',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/font-size',
        browserSupport: { chrome: '1', firefox: '1', safari: '1', edge: '12', hasLimitedSupport: false }
    }],
    ['font-weight', {
        propertyName: 'font-weight',
        description: 'Sets the weight (or boldness) of the font.',
        syntax: 'normal | bold | bolder | lighter | <number>',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/font-weight',
        browserSupport: { chrome: '1', firefox: '1', safari: '1', edge: '12', hasLimitedSupport: false }
    }],
    ['text-align', {
        propertyName: 'text-align',
        description: 'Sets the horizontal alignment of the inline-level content.',
        syntax: 'start | end | left | right | center | justify',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/text-align',
        browserSupport: { chrome: '1', firefox: '1', safari: '1', edge: '12', hasLimitedSupport: false }
    }],
    ['line-height', {
        propertyName: 'line-height',
        description: 'Sets the height of a line box.',
        syntax: 'normal | <number> | <length> | <percentage>',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/line-height',
        browserSupport: { chrome: '1', firefox: '1', safari: '1', edge: '12', hasLimitedSupport: false }
    }],

    // Colors
    ['color', {
        propertyName: 'color',
        description: 'Sets the foreground color value of an element\'s text and text decorations.',
        syntax: '<color>',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/color',
        browserSupport: { chrome: '1', firefox: '1', safari: '1', edge: '12', hasLimitedSupport: false }
    }],
    ['background-color', {
        propertyName: 'background-color',
        description: 'Sets the background color of an element.',
        syntax: '<color>',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/background-color',
        browserSupport: { chrome: '1', firefox: '1', safari: '1', edge: '12', hasLimitedSupport: false }
    }],

    // Borders
    ['border', {
        propertyName: 'border',
        description: 'Shorthand property for border-width, border-style, and border-color.',
        syntax: '<line-width> || <line-style> || <color>',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/border',
        browserSupport: { chrome: '1', firefox: '1', safari: '1', edge: '12', hasLimitedSupport: false }
    }],
    ['border-radius', {
        propertyName: 'border-radius',
        description: 'Rounds the corners of an element\'s outer border edge.',
        syntax: '<length-percentage>{1,4} [ / <length-percentage>{1,4} ]?',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/border-radius',
        browserSupport: { chrome: '4', firefox: '4', safari: '5', edge: '12', hasLimitedSupport: false }
    }],

    // Effects
    ['box-shadow', {
        propertyName: 'box-shadow',
        description: 'Adds shadow effects around an element\'s frame.',
        syntax: 'none | <shadow>#',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/box-shadow',
        browserSupport: { chrome: '10', firefox: '4', safari: '5.1', edge: '12', hasLimitedSupport: false }
    }],
    ['opacity', {
        propertyName: 'opacity',
        description: 'Sets the opacity of an element.',
        syntax: '<alpha-value>',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/opacity',
        browserSupport: { chrome: '1', firefox: '1', safari: '2', edge: '12', hasLimitedSupport: false }
    }],

    // Transforms
    ['transform', {
        propertyName: 'transform',
        description: 'Lets you rotate, scale, skew, or translate an element.',
        syntax: 'none | <transform-list>',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/transform',
        browserSupport: { chrome: '36', firefox: '16', safari: '9', edge: '12', hasLimitedSupport: false }
    }],

    // Transitions
    ['transition', {
        propertyName: 'transition',
        description: 'Shorthand property for transition-property, transition-duration, transition-timing-function, and transition-delay.',
        syntax: '<single-transition>#',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/transition',
        browserSupport: { chrome: '26', firefox: '16', safari: '9', edge: '12', hasLimitedSupport: false }
    }],

    // Position
    ['position', {
        propertyName: 'position',
        description: 'Sets how an element is positioned in a document.',
        syntax: 'static | relative | absolute | fixed | sticky',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/position',
        browserSupport: { chrome: '1', firefox: '1', safari: '1', edge: '12', hasLimitedSupport: false }
    }],
    ['top', {
        propertyName: 'top',
        description: 'Specifies the vertical position of a positioned element.',
        syntax: 'auto | <length> | <percentage>',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/top',
        browserSupport: { chrome: '1', firefox: '1', safari: '1', edge: '12', hasLimitedSupport: false }
    }],
    ['right', {
        propertyName: 'right',
        description: 'Specifies the horizontal position of a positioned element.',
        syntax: 'auto | <length> | <percentage>',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/right',
        browserSupport: { chrome: '1', firefox: '1', safari: '1', edge: '12', hasLimitedSupport: false }
    }],
    ['bottom', {
        propertyName: 'bottom',
        description: 'Specifies the vertical position of a positioned element.',
        syntax: 'auto | <length> | <percentage>',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/bottom',
        browserSupport: { chrome: '1', firefox: '1', safari: '1', edge: '12', hasLimitedSupport: false }
    }],
    ['left', {
        propertyName: 'left',
        description: 'Specifies the horizontal position of a positioned element.',
        syntax: 'auto | <length> | <percentage>',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/left',
        browserSupport: { chrome: '1', firefox: '1', safari: '1', edge: '12', hasLimitedSupport: false }
    }],
    ['z-index', {
        propertyName: 'z-index',
        description: 'Sets the z-order of a positioned element and its descendants.',
        syntax: 'auto | <integer>',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/z-index',
        browserSupport: { chrome: '1', firefox: '1', safari: '1', edge: '12', hasLimitedSupport: false }
    }],

    // Overflow
    ['overflow', {
        propertyName: 'overflow',
        description: 'Shorthand property for overflow-x and overflow-y.',
        syntax: 'visible | hidden | clip | scroll | auto',
        mdnUrl: 'https://developer.mozilla.org/en-US/docs/Web/CSS/overflow',
        browserSupport: { chrome: '1', firefox: '1', safari: '1', edge: '12', hasLimitedSupport: false }
    }],
]);

/**
 * Get MDN property info by property name.
 * 
 * @param propertyName - The CSS property name
 * @returns The MDN property info, or undefined if not found
 */
export function getMDNPropertyInfo(propertyName: string): MDNPropertyInfo | undefined {
    return mdnProperties.get(propertyName.toLowerCase());
}
