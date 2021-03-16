use crate::{TestError, TestResult};

use super::*;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct SqlServerConnectorTag {
    version: Option<SqlServerVersion>,
}

impl ConnectorTagInterface for SqlServerConnectorTag {
    fn connection_string(&self) -> String {
        todo!()
    }

    fn capabilities(&self) -> Vec<ConnectorCapability> {
        todo!()
    }

    fn as_parse_pair(&self) -> (String, Option<String>) {
        let version = self.version.as_ref().map(ToString::to_string);
        ("sqlserver".to_owned(), version)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SqlServerVersion {
    V2017,
    V2019,
}

impl SqlServerConnectorTag {
    pub fn new(version: Option<&str>) -> TestResult<Self> {
        let version = match version {
            Some(v) => Some(SqlServerVersion::try_from(v)?),
            None => None,
        };

        Ok(Self { version })
    }

    /// Returns all versions of this connector.
    pub fn all() -> Vec<Self> {
        vec![
            Self {
                version: Some(SqlServerVersion::V2017),
            },
            Self {
                version: Some(SqlServerVersion::V2019),
            },
        ]
    }
}

impl TryFrom<&str> for SqlServerVersion {
    type Error = TestError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let version = match s {
            "2017" => Self::V2017,
            "2019" => Self::V2019,
            _ => return Err(TestError::parse_error(format!("Unknown SqlServer version `{}`", s))),
        };

        Ok(version)
    }
}

impl ToString for SqlServerVersion {
    fn to_string(&self) -> String {
        match self {
            SqlServerVersion::V2017 => "2017",
            SqlServerVersion::V2019 => "2019",
        }
        .to_owned()
    }
}