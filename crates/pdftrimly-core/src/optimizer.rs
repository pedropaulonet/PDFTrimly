use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    time::{Duration, Instant},
};

use lopdf::{Document, LoadOptions};
use tempfile::NamedTempFile;
use thiserror::Error;

use crate::{images, streams, validation, CompressionLevel};

/// Bounds for this in-process proof of concept, not a sandbox for hostile PDFs.
#[derive(Debug, Clone)]
pub struct OptimizeOptions {
    pub level: CompressionLevel,
    pub max_input_bytes: u64,
    pub max_stream_bytes: usize,
    pub max_total_decoded_bytes: usize,
    pub max_image_pixels: u64,
    pub max_image_decoded_bytes: usize,
    pub max_total_image_decoded_bytes: usize,
}

impl Default for OptimizeOptions {
    fn default() -> Self {
        Self {
            level: CompressionLevel::Normal,
            max_input_bytes: 64 * 1024 * 1024,
            max_stream_bytes: 32 * 1024 * 1024,
            max_total_decoded_bytes: 128 * 1024 * 1024,
            max_image_pixels: 32_000_000,
            max_image_decoded_bytes: 128 * 1024 * 1024,
            max_total_image_decoded_bytes: 512 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Reduced,
    Unchanged,
}

#[derive(Debug)]
pub struct OptimizeReport {
    pub outcome: Outcome,
    pub input_bytes: u64,
    pub output_bytes: u64,
    pub pages: usize,
    /// Streams in the published output that were actually optimized.
    pub optimized_streams: usize,
    pub optimized_images: usize,
    pub skipped_images: usize,
    pub skipped_streams: usize,
    pub duration: Duration,
}

impl OptimizeReport {
    pub fn savings_percent(&self) -> f64 {
        if self.input_bytes == 0 {
            return 0.0;
        }
        self.input_bytes.saturating_sub(self.output_bytes) as f64 * 100.0 / self.input_bytes as f64
    }
}

#[derive(Debug, Error)]
pub enum OptimizeError {
    #[error("O arquivo de saída já existe; escolha outro caminho")]
    OutputExists,
    #[error("PDF criptografado não é suportado nesta etapa")]
    Encrypted,
    #[error("PDF com assinatura ou campo de assinatura não será alterado")]
    Signed,
    #[error("Limite de processamento excedido: {0}")]
    LimitExceeded(&'static str),
    #[error("Estrutura PDF inválida ou não suportada: {0}")]
    Structure(&'static str),
    #[error("Não foi possível ler a estrutura PDF")]
    Pdf(#[source] lopdf::Error),
    #[error("Falha de acesso ao arquivo: {0}")]
    Io(#[from] std::io::Error),
}

pub(crate) fn load(bytes: &[u8], options: &OptimizeOptions) -> Result<Document, OptimizeError> {
    Document::load_mem_with_options(
        bytes,
        LoadOptions {
            strict: true,
            max_decompressed_size: Some(options.max_stream_bytes),
            ..Default::default()
        },
    )
    .map_err(|error| match error {
        lopdf::Error::InvalidPassword | lopdf::Error::Decryption(_) => OptimizeError::Encrypted,
        other => OptimizeError::Pdf(other),
    })
}

/// Optimize supported streams and publish a validated output without overwrite.
/// If there is no size reduction, the output is a byte-identical original copy.
pub fn optimize(
    input: &Path,
    output: &Path,
    options: &OptimizeOptions,
) -> Result<OptimizeReport, OptimizeError> {
    let started = Instant::now();
    // symlink_metadata also detects dangling symlinks at the destination.
    match fs::symlink_metadata(output) {
        Ok(_) => return Err(OptimizeError::OutputExists),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let file = File::open(input)?;
    if !file.metadata()?.is_file() {
        return Err(OptimizeError::Structure(
            "a entrada deve ser um arquivo regular",
        ));
    }
    if file.metadata()?.len() > options.max_input_bytes {
        return Err(OptimizeError::LimitExceeded(
            "tamanho do arquivo de entrada",
        ));
    }
    let mut original = Vec::new();
    file.take(options.max_input_bytes.saturating_add(1))
        .read_to_end(&mut original)?;
    if original.len() as u64 > options.max_input_bytes {
        return Err(OptimizeError::LimitExceeded(
            "tamanho do arquivo de entrada",
        ));
    }
    let mut document = load(&original, options)?;
    let pages = validation::validate_input(&document)?;
    let image_stats = images::optimize_images(&mut document, options);
    let stats = streams::optimize_streams(&mut document, options)?;

    let parent = output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = NamedTempFile::new_in(parent)?;
    let mut outcome = Outcome::Unchanged;
    if stats.optimized > 0 || image_stats.optimized > 0 {
        // Write classic xref tables; old xref/object streams are storage objects.
        validation::prepare_for_save(&mut document);
        document.save_to(temporary.as_file_mut())?;
        temporary.as_file_mut().flush()?;
        if temporary.as_file().metadata()?.len() < original.len() as u64 {
            let mut candidate = Vec::new();
            temporary.as_file_mut().seek(SeekFrom::Start(0))?;
            temporary.as_file_mut().read_to_end(&mut candidate)?;
            let reloaded = load(&candidate, options)?;
            validation::validate_output(&document, &reloaded, pages)?;
            outcome = Outcome::Reduced;
        }
    }
    if outcome == Outcome::Unchanged {
        temporary.as_file_mut().set_len(0)?;
        temporary.as_file_mut().seek(SeekFrom::Start(0))?;
        temporary.as_file_mut().write_all(&original)?;
    }
    temporary.as_file_mut().sync_all()?;
    let output_bytes = temporary.as_file().metadata()?.len();
    temporary.persist_noclobber(output).map_err(|error| {
        if error.error.kind() == std::io::ErrorKind::AlreadyExists {
            OptimizeError::OutputExists
        } else {
            OptimizeError::Io(error.error)
        }
    })?;
    Ok(OptimizeReport {
        outcome,
        input_bytes: original.len() as u64,
        output_bytes,
        pages,
        optimized_streams: if outcome == Outcome::Reduced {
            stats.optimized
        } else {
            0
        },
        skipped_streams: stats.skipped,
        optimized_images: if outcome == Outcome::Reduced {
            image_stats.optimized
        } else {
            0
        },
        skipped_images: image_stats.skipped
            + if outcome == Outcome::Unchanged {
                image_stats.optimized
            } else {
                0
            },
        duration: started.elapsed(),
    })
}
