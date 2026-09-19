import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import ar from './ar.json';
import en from './en.json';
import { STORAGE_KEY, DEFAULT_LANGUAGE } from './config';

const resources = {
  ar: { translation: ar },
  en: { translation: en },
};

const savedLanguage = localStorage.getItem(STORAGE_KEY) as 'ar' | 'en' | null;
const initialLanguage = savedLanguage || DEFAULT_LANGUAGE;

i18n
  .use(initReactI18next)
  .init({
    resources,
    lng: initialLanguage,
    fallbackLng: DEFAULT_LANGUAGE,
    interpolation: {
      escapeValue: false,
    },
    react: {
      useSuspense: false,
    },
  });

export const changeLanguage = (lang: 'ar' | 'en') => {
  localStorage.setItem(STORAGE_KEY, lang);
  i18n.changeLanguage(lang);
  document.documentElement.lang = lang;
  document.documentElement.dir = lang === 'ar' ? 'rtl' : 'ltr';
};

export default i18n;