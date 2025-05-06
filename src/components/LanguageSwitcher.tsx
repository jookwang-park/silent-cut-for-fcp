import { useTranslation } from "react-i18next";
import { GlobeIcon, CheckIcon } from "lucide-react";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";

const LANGUAGES = [
  { code: "en", label: "English" },
  { code: "ko", label: "한국어" },
];

export function LanguageSwitcher() {
  const { i18n, t } = useTranslation();

  const changeLanguage = (lng: string) => {
    i18n.changeLanguage(lng);
  };

  const currentLanguage = i18n.language;

  return (
    <DropdownMenu>
      <DropdownMenuTrigger>
        <div className="flex items-center gap-1">
          <GlobeIcon className="h-4 w-4" />
          <span className="hidden sm:inline-block">
            {LANGUAGES.find((lang) => lang.code === currentLanguage)?.label ||
              LANGUAGES[0].label}
          </span>
        </div>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        {LANGUAGES.map((language) => (
          <DropdownMenuItem
            key={language.code}
            onClick={() => changeLanguage(language.code)}
            className="flex items-center gap-2"
          >
            {language.code === currentLanguage && (
              <CheckIcon className="h-4 w-4" />
            )}
            <span
              className={language.code === currentLanguage ? "font-medium" : ""}
            >
              {t(`language.${language.code}`)}
            </span>
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
