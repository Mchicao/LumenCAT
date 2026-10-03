use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub type Result<T> = std::result::Result<T, CatError>;

#[derive(Debug, thiserror::Error)]
pub enum CatError {
    #[error("Error de archivo: {0}")]
    Io(#[from] std::io::Error),
    #[error("Error de base de datos: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Formato no soportado o inválido: {0}")]
    Format(String),
    #[error("Operación rechazada: {0}")]
    Invalid(String),
    #[error("Operación cancelada; no se aplicaron cambios")]
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentFormat {
    Txt,
    Xliff12,
    Docx,
}
impl DocumentFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Txt => "txt",
            Self::Xliff12 => "xliff12",
            Self::Docx => "docx",
        }
    }
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "txt" => Ok(Self::Txt),
            "xliff12" => Ok(Self::Xliff12),
            "docx" => Ok(Self::Docx),
            _ => Err(CatError::Format(format!("formato {value}"))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentState {
    Draft,
    Confirmed,
}
impl SegmentState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Confirmed => "confirmed",
        }
    }
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "draft" => Ok(Self::Draft),
            "confirmed" => Ok(Self::Confirmed),
            _ => Err(CatError::Invalid("estado desconocido".into())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Human,
    TranslationMemory,
    Imported,
}
impl Origin {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::TranslationMemory => "tm",
            Self::Imported => "imported",
        }
    }
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "human" => Ok(Self::Human),
            "tm" => Ok(Self::TranslationMemory),
            "imported" => Ok(Self::Imported),
            _ => Err(CatError::Invalid("origen desconocido".into())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImportedSegment {
    pub external_id: String,
    pub source: String,
    pub target: String,
    pub state: SegmentState,
    pub locked: bool,
}
#[derive(Debug, Clone)]
pub struct ImportedDocument {
    pub name: String,
    pub format: DocumentFormat,
    pub original: Vec<u8>,
    pub original_path: Option<std::path::PathBuf>,
    pub source_lang: String,
    pub target_lang: String,
    pub segments: Vec<ImportedSegment>,
}
#[derive(Debug, Clone)]
pub struct Segment {
    pub id: i64,
    pub document_id: i64,
    pub ordinal: usize,
    pub external_id: String,
    pub source: String,
    pub target: String,
    pub state: SegmentState,
    pub locked: bool,
    pub origin: Origin,
    pub revision: i64,
}
#[derive(Debug, Clone)]
pub struct DocumentInfo {
    pub id: i64,
    pub name: String,
    pub format: DocumentFormat,
    pub segment_count: usize,
    pub source_lang: String,
    pub target_lang: String,
}
#[derive(Debug, Clone)]
pub struct EditCommand {
    pub segment_id: i64,
    pub expected_revision: i64,
    pub target: String,
    pub state: SegmentState,
    pub locked: bool,
    pub origin: Origin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSettings {
    pub source_lang: String,
    pub target_lang: String,
}

impl ProjectSettings {
    pub fn validate(&self) -> Result<()> {
        validate_language(&self.source_lang)?;
        validate_language(&self.target_lang)?;
        if self.source_lang.eq_ignore_ascii_case(&self.target_lang) {
            return Err(CatError::Invalid(
                "Elige idiomas de origen y destino diferentes".into(),
            ));
        }
        Ok(())
    }
}

pub fn validate_language(language: &str) -> Result<()> {
    let mut subtags = language.split('-');
    let primary = subtags.next().unwrap_or_default();
    if language.len() > 63
        || !(2..=8).contains(&primary.len())
        || !primary.bytes().all(|b| b.is_ascii_alphabetic())
        || subtags.any(|part| {
            part.is_empty() || part.len() > 8 || !part.bytes().all(|b| b.is_ascii_alphanumeric())
        })
    {
        return Err(CatError::Invalid(
            "Idioma inválido: usa un código como fr, es, pt-BR o zh-Hant".into(),
        ));
    }
    Ok(())
}
#[derive(Debug, Clone)]
pub struct TmUnit {
    pub source: String,
    pub target: String,
    pub source_lang: String,
    pub target_lang: String,
    pub raw_xml: String,
}
#[derive(Debug, Clone)]
pub struct TmMatch {
    pub id: i64,
    pub source: String,
    pub target: String,
    pub score: f64,
    pub exact: bool,
}
#[derive(Debug, Clone)]
pub struct QaIssue {
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
    pub fn check(&self) -> Result<()> {
        if self.0.load(Ordering::Relaxed) {
            Err(CatError::Cancelled)
        } else {
            Ok(())
        }
    }
}
