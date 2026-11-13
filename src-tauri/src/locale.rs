#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    Chinese,
    English,
}

impl Language {
    #[cfg(any(target_os = "macos", test))]
    fn from_tag(tag: &str) -> Self {
        if tag
            .split(['-', '_'])
            .next()
            .unwrap_or("")
            .eq_ignore_ascii_case("zh")
        {
            Self::Chinese
        } else {
            Self::English
        }
    }

    pub fn system() -> Self {
        #[cfg(target_os = "macos")]
        {
            objc2_foundation::NSLocale::preferredLanguages()
                .firstObject()
                .map(|tag| Self::from_tag(&tag.to_string()))
                .unwrap_or(Self::English)
        }
        #[cfg(windows)]
        {
            #[link(name = "kernel32")]
            extern "system" {
                #[link_name = "GetUserDefaultUILanguage"]
                fn user_ui_language() -> u16;
            }
            if unsafe { user_ui_language() } & 0x03ff == 0x04 {
                Self::Chinese
            } else {
                Self::English
            }
        }
        #[cfg(not(any(target_os = "macos", windows)))]
        {
            Self::English
        }
    }

    pub fn text(self, chinese: &'static str, english: &'static str) -> &'static str {
        match self {
            Self::Chinese => chinese,
            Self::English => english,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chinese_system_language_variants_use_chinese() {
        for tag in ["zh", "zh-CN", "zh-Hans-CN", "zh-Hant-TW", "ZH_HK"] {
            assert_eq!(Language::from_tag(tag), Language::Chinese);
        }
    }

    #[test]
    fn other_system_languages_fall_back_to_english() {
        for tag in ["en", "en-US", "ja-JP", "fr-FR", "", "zhang"] {
            assert_eq!(Language::from_tag(tag), Language::English);
        }
    }

    #[test]
    fn each_label_uses_only_the_selected_language() {
        assert_eq!(Language::Chinese.text("游戏", "Game"), "游戏");
        assert_eq!(Language::English.text("游戏", "Game"), "Game");
    }
}
