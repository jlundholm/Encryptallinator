mod crypto;

use std::{
    fs,
    path::{Path, PathBuf},
};

use crypto::{decrypt_payload, encrypt_payload, Payload, PayloadKind};
use serde::{Deserialize, Serialize};
use tar::{Archive, Builder, EntryType};
use thiserror::Error;
use walkdir::WalkDir;

const ENCRYPTED_EXTENSION: &str = "encryptallinator";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum OperationMode {
    Encrypt,
    Decrypt,
}

#[derive(Debug, Deserialize)]
struct ProcessRequest {
    path: String,
    password: String,
    mode: OperationMode,
}

#[derive(Debug, Serialize)]
struct ProcessResponse {
    output_path: String,
    output_kind: &'static str,
    message: String,
}

#[derive(Debug, Error)]
enum AppError {
    #[error("Enter a password before continuing.")]
    MissingPassword,
    #[error("Select a file or folder before continuing.")]
    MissingPath,
    #[error("The selected path does not exist.")]
    PathDoesNotExist,
    #[error("Decrypt expects an Encryptallinator file, not a folder.")]
    DecryptRequiresFile,
    #[error("Only files and folders are supported.")]
    UnsupportedInputType,
    #[error("Folder encryption does not support symbolic links.")]
    UnsupportedSymlink,
    #[error("The encrypted file name or metadata is invalid.")]
    InvalidStoredName,
    #[error("The encrypted archive contains an unsafe path.")]
    UnsafeArchivePath,
    #[error("Unable to determine a file or folder name for the selected path.")]
    MissingFileName,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Crypto(#[from] crypto::CryptoError),
    #[error("Failed to walk the selected folder: {0}")]
    WalkDir(#[from] walkdir::Error),
}

#[tauri::command]
fn process_item(request: ProcessRequest) -> Result<ProcessResponse, String> {
    process_item_impl(request).map_err(|error| error.to_string())
}

fn process_item_impl(request: ProcessRequest) -> Result<ProcessResponse, AppError> {
    if request.password.is_empty() {
        return Err(AppError::MissingPassword);
    }

    if request.path.is_empty() {
        return Err(AppError::MissingPath);
    }

    let input_path = PathBuf::from(&request.path);
    if !input_path.exists() {
        return Err(AppError::PathDoesNotExist);
    }

    match request.mode {
        OperationMode::Encrypt => encrypt_selected_path(&input_path, &request.password),
        OperationMode::Decrypt => decrypt_selected_path(&input_path, &request.password),
    }
}

fn encrypt_selected_path(input_path: &Path, password: &str) -> Result<ProcessResponse, AppError> {
    let payload = if input_path.is_file() {
        Payload {
            kind: PayloadKind::File,
            original_name: single_component_name(input_path)?,
            data: fs::read(input_path)?,
        }
    } else if input_path.is_dir() {
        Payload {
            kind: PayloadKind::DirectoryArchive,
            original_name: single_component_name(input_path)?,
            data: archive_directory(input_path)?,
        }
    } else {
        return Err(AppError::UnsupportedInputType);
    };

    let encrypted_bytes = encrypt_payload(&payload, password)?;
    let output_path = next_available_path(encrypted_output_path(input_path)?);
    fs::write(&output_path, encrypted_bytes)?;

    Ok(ProcessResponse {
        output_path: output_path.to_string_lossy().into_owned(),
        output_kind: "file",
        message: format!("Encrypted item written to {}.", output_path.display()),
    })
}

fn decrypt_selected_path(input_path: &Path, password: &str) -> Result<ProcessResponse, AppError> {
    if !input_path.is_file() {
        return Err(AppError::DecryptRequiresFile);
    }

    let encrypted_bytes = fs::read(input_path)?;
    let payload = decrypt_payload(&encrypted_bytes, password)?;
    let safe_name = validated_name(&payload.original_name)?;
    let output_parent = input_path.parent().ok_or(AppError::MissingFileName)?;

    match payload.kind {
        PayloadKind::File => {
            let output_path = next_available_path(output_parent.join(safe_name));
            fs::write(&output_path, payload.data)?;

            Ok(ProcessResponse {
                output_path: output_path.to_string_lossy().into_owned(),
                output_kind: "file",
                message: format!("Decrypted file written to {}.", output_path.display()),
            })
        }
        PayloadKind::DirectoryArchive => {
            let output_path = next_available_path(output_parent.join(safe_name));
            fs::create_dir_all(&output_path)?;
            unpack_archive(&payload.data, &output_path)?;

            Ok(ProcessResponse {
                output_path: output_path.to_string_lossy().into_owned(),
                output_kind: "folder",
                message: format!("Decrypted folder restored to {}.", output_path.display()),
            })
        }
    }
}

fn encrypted_output_path(input_path: &Path) -> Result<PathBuf, AppError> {
    let file_name = single_component_name(input_path)?;
    let parent = input_path.parent().ok_or(AppError::MissingFileName)?;
    Ok(parent.join(format!("{file_name}.{ENCRYPTED_EXTENSION}")))
}

fn single_component_name(path: &Path) -> Result<String, AppError> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(AppError::MissingFileName)?;

    validated_name(name).map(ToOwned::to_owned)
}

fn validated_name(name: &str) -> Result<&str, AppError> {
    let path = Path::new(name);
    if name.trim().is_empty() {
        return Err(AppError::InvalidStoredName);
    }

    let mut components = path.components();
    match (components.next(), components.next()) {
        (Some(std::path::Component::Normal(_)), None) => Ok(name),
        _ => Err(AppError::InvalidStoredName),
    }
}

fn next_available_path(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }

    let parent = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("encryptallinator");
    let extension = path.extension().and_then(|value| value.to_str());

    for index in 1.. {
        let candidate_name = match extension {
            Some(extension) => format!("{stem} ({index}).{extension}"),
            None => format!("{stem} ({index})"),
        };
        let candidate = parent.join(candidate_name);
        if !candidate.exists() {
            return candidate;
        }
    }

    unreachable!("path search should always find an available candidate")
}

fn archive_directory(path: &Path) -> Result<Vec<u8>, AppError> {
    let mut buffer = Vec::new();
    let mut builder = Builder::new(&mut buffer);

    for entry in WalkDir::new(path) {
        let entry = entry?;
        let entry_path = entry.path();

        if entry_path == path {
            continue;
        }

        let relative_path = entry_path
            .strip_prefix(path)
            .map_err(|_| AppError::UnsafeArchivePath)?;

        if entry.file_type().is_symlink() {
            return Err(AppError::UnsupportedSymlink);
        }

        if entry.file_type().is_dir() {
            builder.append_dir(relative_path, entry_path)?;
        } else if entry.file_type().is_file() {
            builder.append_path_with_name(entry_path, relative_path)?;
        } else {
            return Err(AppError::UnsupportedInputType);
        }
    }

    builder.finish()?;
    drop(builder);
    Ok(buffer)
}

fn unpack_archive(archive_bytes: &[u8], output_path: &Path) -> Result<(), AppError> {
    let cursor = std::io::Cursor::new(archive_bytes);
    let mut archive = Archive::new(cursor);

    for entry_result in archive.entries()? {
        let mut entry = entry_result?;
        let relative_path = entry.path()?.into_owned();
        ensure_safe_archive_path(&relative_path)?;

        let destination = output_path.join(&relative_path);
        match entry.header().entry_type() {
            EntryType::Directory => fs::create_dir_all(&destination)?,
            EntryType::Regular => {
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent)?;
                }
                entry.unpack(&destination)?;
            }
            _ => return Err(AppError::UnsupportedSymlink),
        }
    }

    Ok(())
}

fn ensure_safe_archive_path(path: &Path) -> Result<(), AppError> {
    for component in path.components() {
        match component {
            std::path::Component::Normal(_) | std::path::Component::CurDir => {}
            _ => return Err(AppError::UnsafeArchivePath),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{archive_directory, unpack_archive, validated_name};
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("encryptallinator-{label}-{nonce}"));
        fs::create_dir_all(&path).expect("temp directory should be created");
        path
    }

    #[test]
    fn directory_archive_round_trip_preserves_nested_files() {
        let root = unique_temp_dir("archive");
        let source = root.join("source");
        let restored = root.join("restored");

        fs::create_dir_all(source.join("nested")).unwrap();
        fs::write(source.join("nested").join("note.txt"), b"secret payload").unwrap();

        let archive = archive_directory(&source).unwrap();
        fs::create_dir_all(&restored).unwrap();
        unpack_archive(&archive, &restored).unwrap();

        assert_eq!(
            fs::read(restored.join("nested").join("note.txt")).unwrap(),
            b"secret payload"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn stored_names_must_be_single_path_components() {
        assert!(validated_name("safe.txt").is_ok());
        assert!(validated_name("nested\\unsafe.txt").is_err());
        assert!(validated_name("../unsafe.txt").is_err());
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![process_item])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
