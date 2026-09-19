export const SUPPORTED_LANGUAGES = ['ar', 'en'] as const;
export type Language = typeof SUPPORTED_LANGUAGES[number];

export const DEFAULT_LANGUAGE: Language = 'ar';
export const STORAGE_KEY = 'vortex-language';