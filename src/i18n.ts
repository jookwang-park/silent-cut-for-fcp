import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import { ModuleType } from "i18next";
import { locale } from "@tauri-apps/plugin-os";

import enTranslation from "./locales/en.json";
import koTranslation from "./locales/ko.json";

const languageDetector = {
  type: "languageDetector" as ModuleType,
  async: true, // flags below detection to be async
  detect: async (callback: any) => {
    const loc = await locale();
    if (!loc) {
      callback("en");
      return;
    }

    const value = loc.split("-")[0];
    switch (value) {
      case "en":
      case "ko":
        callback(value);
        break;
      default:
        callback("en");
    }
  },
  init: () => {},
  cacheUserLanguage: () => {},
};

i18n
  .use(languageDetector)
  .use(initReactI18next)
  .init({
    resources: {
      en: {
        translation: enTranslation,
      },
      ko: {
        translation: koTranslation,
      },
    },
    fallbackLng: "en",
    debug: false,

    interpolation: {
      escapeValue: false,
    },

    load: "languageOnly",

    detection: {
      order: ["platformLanguageDetector", "localStorage", "navigator"],
      lookupLocalStorage: "i18nextLng",
      caches: ["localStorage"],
    },
  });

export default i18n;
