export type Category =
  | 'Software'
  | 'Library'
  | 'Framework'
  | 'Design'
  | 'AI'
  | 'Platform'
  | 'Hardware'
  | 'Social'
  | 'Google'
  | 'Privacy'
  | 'Communications'
  | 'Education'
  | 'Database'
  | 'Hosting'
  | 'Payment'
  | 'Analytics'
  | 'Security'
  | 'DevOps'
  | 'Cloud'
  | 'Language'
  | 'Browser'
  | 'Gaming'
  | 'Finance'
  | 'Blockchain';

export type ThemeOptions = {
  dark: string;
  light: string;
};

export interface iSVG {
  id?: number;
  title: string;
  category: Category | Category[];
  route: string | ThemeOptions;
  wordmark?: string | ThemeOptions;
  brandUrl?: string;
  url: string;
}
