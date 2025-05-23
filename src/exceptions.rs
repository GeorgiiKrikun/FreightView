use bollard::errors::Error as DockerError;

#[allow(unused)]
#[derive(Debug, thiserror::Error)]
pub enum ImageParcingError {
    CantDownloadImage,
    CantGetAHomeDir,
    FilesystemError,
    JSONError,
    DockerAPIError,
    LayerParsingError,
    NonUnixFileSystem,
    UnparceableFileName,
}

#[derive(Debug, thiserror::Error)]
pub enum GUIError {
    CantFilterTree,
}

impl std::fmt::Display for GUIError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::fmt::Display for ImageParcingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<std::io::Error> for ImageParcingError {
    fn from(_err: std::io::Error) -> ImageParcingError {
        ImageParcingError::FilesystemError
    }
}

impl From<serde_json::Error> for ImageParcingError {
    fn from(_err: serde_json::Error) -> ImageParcingError {
        ImageParcingError::JSONError
    }
}

impl From<DockerError> for ImageParcingError {
    fn from(_err: DockerError) -> ImageParcingError {
        ImageParcingError::DockerAPIError
    }
}
