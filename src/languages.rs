use lazy_static::lazy_static;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Language {
    pub id: String,
    pub name: String,
    pub native_name: String,
}

lazy_static! {
    static ref LANGUAGES: Vec<Language> = vec![
        Language { id: "en".to_string(), name: "English".to_string(), native_name: "English".to_string() },
        Language { id: "es".to_string(), name: "Spanish".to_string(), native_name: "Español".to_string() },
        Language { id: "fr".to_string(), name: "French".to_string(), native_name: "Français".to_string() },
        Language { id: "de".to_string(), name: "German".to_string(), native_name: "Deutsch".to_string() },
        Language { id: "it".to_string(), name: "Italian".to_string(), native_name: "Italiano".to_string() },
        Language { id: "pt".to_string(), name: "Portuguese".to_string(), native_name: "Português".to_string() },
        Language { id: "ru".to_string(), name: "Russian".to_string(), native_name: "Русский".to_string() },
        Language { id: "zh-TW".to_string(), name: "Traditional Chinese".to_string(), native_name: "正體中文".to_string() },
        Language { id: "zh-CN".to_string(), name: "Simplified Chinese".to_string(), native_name: "简体中文".to_string() },
        Language { id: "ja".to_string(), name: "Japanese".to_string(), native_name: "日本語".to_string() },
        Language { id: "ko".to_string(), name: "Korean".to_string(), native_name: "한국어".to_string() },
        Language { id: "ar".to_string(), name: "Arabic".to_string(), native_name: "العربية".to_string() },
        Language { id: "hi".to_string(), name: "Hindi".to_string(), native_name: "हिन्दी".to_string() },
    ];

    static ref LANGUAGE_MAP: HashMap<String, Language> = {
        let mut map = HashMap::new();
        for lang in LANGUAGES.iter() {
            map.insert(lang.id.clone(), lang.clone());
        }
        map
    };
}

pub const DEFAULT_LANGUAGE_ID: &str = "en";

pub fn get_all_languages() -> Vec<Language> {
    LANGUAGES.clone()
}

pub fn get_language_by_id(id: &str) -> Language {
    LANGUAGE_MAP.get(id).cloned().unwrap_or_else(|| LANGUAGES[0].clone())
}

pub fn get_translation_prompt(language_id: &str) -> String {
    if language_id == "en" {
        return String::new();
    }
    
    let language = get_language_by_id(language_id);
    
    format!(
        r#"
Regardless of the instructions above, you MUST format your responses as follows:

1. First, provide your answer in English, enclosed in markdown blockquote format (> Your English response here)
2. Then, provide the translated version in {} ({}) as regular text.

You MUST use this exact format for every response:

> [English original answer here]

[{} translation here]

Do not include any additional explanations or notes about the translation process.
If you're unsure about any specialized terms, use the most appropriate translation for the context.
"#,
        language.name, language.native_name, language.name
    )
} 