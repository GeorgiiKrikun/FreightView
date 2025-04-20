use bollard::errors::Error as DockerError;
use std::io::Error;
use thiserror::Error;

#[derive(Debug, thiserror::Error)]
pub enum ImageParcingError {
    ImageNotFound,
    CantGetAHomeDir,
    FilesystemError,
    JSONError,
    DockerAPIError,
    LayerParsingError,
}

impl std::fmt::Display for ImageParcingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<std::io::Error> for ImageParcingError {
    fn from(err: std::io::Error) -> ImageParcingError {
        ImageParcingError::FilesystemError
    }
}

impl From<serde_json::Error> for ImageParcingError {
    fn from(err: serde_json::Error) -> ImageParcingError {
        ImageParcingError::JSONError
    }
}

impl From<DockerError> for ImageParcingError {
    fn from(err: DockerError) -> ImageParcingError {
        ImageParcingError::DockerAPIError
    }
}
