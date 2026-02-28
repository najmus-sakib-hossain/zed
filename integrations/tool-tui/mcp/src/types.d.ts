declare module 'screenshot-desktop' {
    export type ScreenshotDesktopFormat = 'png' | 'jpg';

    export interface ScreenshotDesktopOptions {
        format?: ScreenshotDesktopFormat;
        filename?: string;
        screen?: number | string;
    }

    export default function screenshotDesktop(
        options?: ScreenshotDesktopOptions
    ): Promise<Buffer>;
}
